use super::config::ScoringConfig;
use super::factors::Effect;
use crate::github::types::PullRequest;

#[derive(Debug, Clone)]
pub struct FactorContribution {
    pub label: String,       // e.g. "Age", "Approvals", "Size"
    pub description: String, // e.g. "+1 per 1h (24 units)", "matched '0' -> x0.5"
    pub before: f64,         // Score before this factor
    pub after: f64,          // Score after this factor
}

#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub base_score: f64,
    pub factors: Vec<FactorContribution>,
}

#[derive(Debug, Clone)]
pub struct ScoreResult {
    pub score: f64,
    pub incomplete: bool,
    pub breakdown: ScoreBreakdown,
}

pub fn calculate_score(pr: &PullRequest, config: &ScoringConfig) -> ScoreResult {
    let base_score = config.base_score.unwrap_or(100.0);
    let mut score = base_score;
    let incomplete = false;
    let mut factors = Vec::new();

    // Apply age factor (always available - created_at always present)
    if let Some(ref age_str) = config.age {
        if let Ok(effect) = Effect::parse(age_str) {
            let before = score;
            let age = pr.age();
            let units = calculate_units(&effect, age);
            score = effect.apply(score, units);

            // Build description for age factor
            let description = match &effect {
                Effect::AddPerUnit(n, _) => format!("{:+} per unit ({} units)", n, units),
                Effect::MultiplyPerUnit(n, _) => format!("x{} per unit ({} units)", n, units),
                Effect::Add(n) => format!("{:+}", n),
                Effect::Multiply(n) => format!("x{}", n),
            };

            factors.push(FactorContribution {
                label: "Age".to_string(),
                description,
                before,
                after: score,
            });
        }
    }

    // Apply approvals factor
    if let Some(ref approvals_str) = config.approvals {
        // For approvals, "per N" means "per N approvals", not per time unit
        // Convert formats like "+10 per 1" or "x2 per 1" to use a dummy time unit for parsing
        // The time unit is ignored; we use approval count as units instead
        let parseable_str = if let Some((effect_part, per_part)) = approvals_str.split_once(" per ")
        {
            // Check if per_part is just a number (no time unit)
            if per_part.trim().chars().all(|c| c.is_numeric() || c == '.') {
                format!("{} per 1sec", effect_part)
            } else {
                approvals_str.clone()
            }
        } else {
            approvals_str.clone()
        };

        if let Ok(effect) = Effect::parse(&parseable_str) {
            let before = score;
            let units = pr.approvals as u64;
            score = effect.apply(score, units);

            let description = format!("{} approvals, effect: {}", pr.approvals, approvals_str);
            factors.push(FactorContribution {
                label: "Approvals".to_string(),
                description,
                before,
                after: score,
            });
        }
    }

    // Apply size factor
    if let Some(ref size_config) = config.size {
        if let Some(ref buckets) = size_config.buckets {
            let size = pr.size();
            let before = score;
            let result = apply_bucket_effect(score, size, buckets, |b| &b.range, |b| &b.effect);
            score = result.score;

            // Only add contribution if a bucket matched
            if let (Some(range), Some(effect)) = (result.matched_range, result.matched_effect) {
                let description = format!("{} lines, matched '{}' -> {}", size, range, effect);
                factors.push(FactorContribution {
                    label: "Size".to_string(),
                    description,
                    before,
                    after: score,
                });
            }
        }
    }

    // Apply label factors (multiple matching labels compound)
    if let Some(ref label_configs) = config.labels {
        for label_config in label_configs {
            if pr
                .labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case(&label_config.name))
            {
                if let Ok(effect) = Effect::parse(&label_config.effect) {
                    let before = score;
                    score = effect.apply(score, 1);
                    factors.push(FactorContribution {
                        label: format!("Label: {}", label_config.name),
                        description: format!(
                            "matched label '{}' -> {}",
                            label_config.name, label_config.effect
                        ),
                        before,
                        after: score,
                    });
                }
            }
        }
    }

    // Apply previously_reviewed factor
    if let Some(ref reviewed_effect_str) = config.previously_reviewed {
        if pr.user_has_reviewed {
            if let Ok(effect) = Effect::parse(reviewed_effect_str) {
                let before = score;
                score = effect.apply(score, 1);
                factors.push(FactorContribution {
                    label: "Previously Reviewed".to_string(),
                    description: format!("You have previously reviewed -> {}", reviewed_effect_str),
                    before,
                    after: score,
                });
            }
        }
    }

    // Apply draft factor
    if let Some(ref draft_effect_str) = config.draft {
        if pr.draft {
            if let Ok(effect) = Effect::parse(draft_effect_str) {
                let before = score;
                score = effect.apply(score, 1);
                factors.push(FactorContribution {
                    label: "Draft".to_string(),
                    description: format!("PR is a draft -> {}", draft_effect_str),
                    before,
                    after: score,
                });
            }
        }
    }

    // Floor at zero
    ScoreResult {
        score: score.max(0.0),
        incomplete,
        breakdown: ScoreBreakdown {
            base_score,
            factors,
        },
    }
}

fn calculate_units(effect: &Effect, age: chrono::Duration) -> u64 {
    if let Some(unit_duration) = effect.unit_duration() {
        let age_secs = age.num_seconds().max(0) as u64;
        let unit_secs = unit_duration.as_secs();
        age_secs.checked_div(unit_secs).unwrap_or(0)
    } else {
        1 // Non-per-unit effects apply once
    }
}

struct BucketResult {
    score: f64,
    matched_range: Option<String>,
    matched_effect: Option<String>,
}

fn apply_bucket_effect<T, F1, F2>(
    score: f64,
    value: u64,
    buckets: &[T],
    get_range: F1,
    get_effect: F2,
) -> BucketResult
where
    F1: Fn(&T) -> &str,
    F2: Fn(&T) -> &str,
{
    use super::factors::RangeOp;

    for bucket in buckets {
        let range_str = get_range(bucket);
        let effect_str = get_effect(bucket);
        if let Ok(range) = RangeOp::parse(range_str) {
            if range.matches(value) {
                if let Ok(effect) = Effect::parse(effect_str) {
                    return BucketResult {
                        score: effect.apply(score, 1),
                        matched_range: Some(range_str.to_string()),
                        matched_effect: Some(effect_str.to_string()),
                    };
                }
            }
        }
    }
    BucketResult {
        score,
        matched_range: None,
        matched_effect: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::{LabelEffect, SizeBucket, SizeConfig};
    use chrono::{Duration as ChronoDuration, Utc};

    fn sample_pr(age_hours: i64, approvals: u32, size: u64) -> PullRequest {
        PullRequest {
            title: "Test PR".to_string(),
            number: 1,
            author: "user".to_string(),
            repo: "owner/repo".to_string(),
            url: "https://github.com/owner/repo/pull/1".to_string(),
            created_at: Utc::now() - ChronoDuration::hours(age_hours),
            updated_at: Utc::now(),
            additions: size / 2,
            deletions: size / 2,
            approvals,
            draft: false,
            labels: vec![],
            user_has_reviewed: false,
            filtered_size: None,
        }
    }

    #[test]
    fn test_base_score_only() {
        let pr = sample_pr(1, 0, 100);
        let result = calculate_score(
            &pr,
            &ScoringConfig {
                base_score: Some(100.0),
                age: None,
                approvals: None,
                size: None,
                labels: None,
                previously_reviewed: None,
                draft: None,
            },
        );
        assert_eq!(result.score, 100.0);
        assert!(!result.incomplete);
    }

    #[test]
    fn test_age_factor_additive() {
        let pr = sample_pr(5, 0, 100);
        let result = calculate_score(
            &pr,
            &ScoringConfig {
                base_score: Some(100.0),
                age: Some("+1 per 1h".to_string()),
                approvals: None,
                size: None,
                labels: None,
                previously_reviewed: None,
                draft: None,
            },
        );
        assert_eq!(result.score, 105.0); // 100 + 5*1
    }

    #[test]
    fn test_score_floors_at_zero() {
        let pr = sample_pr(1, 0, 100);
        let result = calculate_score(
            &pr,
            &ScoringConfig {
                base_score: Some(10.0),
                age: Some("+-20 per 1h".to_string()), // Would go negative
                approvals: None,
                size: None,
                labels: None,
                previously_reviewed: None,
                draft: None,
            },
        );
        assert_eq!(result.score, 0.0);
    }

    #[test]
    fn test_approvals_flat_effect() {
        let pr = sample_pr(1, 0, 100);
        let result = calculate_score(
            &pr,
            &ScoringConfig {
                base_score: Some(100.0),
                age: None,
                approvals: Some("x0.5".to_string()),
                size: None,
                labels: None,
                previously_reviewed: None,
                draft: None,
            },
        );
        assert_eq!(result.score, 50.0);
    }

    #[test]
    fn test_size_bucket() {
        let pr = sample_pr(1, 0, 50);
        let result = calculate_score(
            &pr,
            &ScoringConfig {
                base_score: Some(100.0),
                age: None,
                approvals: None,
                size: Some(SizeConfig {
                    exclude: None,
                    buckets: Some(vec![SizeBucket {
                        range: "<100".to_string(),
                        effect: "x2".to_string(),
                    }]),
                }),
                labels: None,
                previously_reviewed: None,
                draft: None,
            },
        );
        assert_eq!(result.score, 200.0);
    }

    #[test]
    fn test_full_scoring_flow() {
        // PR: 24h old, 1 approval, 150 lines
        let pr = sample_pr(24, 1, 150);

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: Some("+1 per 1h".to_string()), // +24 for age
            approvals: Some("x1.5 per 1".to_string()), // x1.5 for 1 approval
            size: Some(SizeConfig {
                exclude: None,
                buckets: Some(vec![
                    SizeBucket {
                        range: "<100".to_string(),
                        effect: "x2".to_string(),
                    },
                    SizeBucket {
                        range: ">=100".to_string(),
                        effect: "x1".to_string(),
                    },
                ]),
            }),
            labels: None,
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);

        // Expected: (100 + 24) * 1.5^1 * 1 = 186
        assert!((result.score - 186.0).abs() < 0.1);
        assert!(!result.incomplete);
    }

    #[test]
    fn test_default_config_scoring() {
        let pr = sample_pr(5, 0, 50);
        let config = ScoringConfig::default();
        let result = calculate_score(&pr, &config);

        // Default config has factors: base=100, +1/h age, +10 per 1 approval (0 approvals = +0), <100 size=x5
        // Expected: (100 + 5 + 0) * 5 = 525
        assert!((result.score - 525.0).abs() < 0.1);
    }

    #[test]
    fn test_multiplicative_age_factor() {
        let pr = sample_pr(3, 0, 100);
        let config = ScoringConfig {
            base_score: Some(100.0),
            age: Some("x1.1 per 1h".to_string()),
            approvals: None,
            size: None,
            labels: None,
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        // 100 * 1.1^3 = 133.1
        assert!((result.score - 133.1).abs() < 0.1);
    }

    #[test]
    fn test_bucket_first_match_wins() {
        let pr = sample_pr(1, 0, 50); // Size 50

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: Some(SizeConfig {
                exclude: None,
                buckets: Some(vec![
                    SizeBucket {
                        range: "<100".to_string(),
                        effect: "x2".to_string(),
                    }, // Matches first
                    SizeBucket {
                        range: "<200".to_string(),
                        effect: "x3".to_string(),
                    }, // Also matches but not used
                ]),
            }),
            labels: None,
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 200.0); // First match (x2), not second (x3)
    }

    #[test]
    fn test_label_factor_additive() {
        let mut pr = sample_pr(1, 0, 100);
        pr.labels = vec!["urgent".to_string()];

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: Some(vec![LabelEffect {
                name: "urgent".to_string(),
                effect: "+10".to_string(),
            }]),
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 110.0);
    }

    #[test]
    fn test_label_factor_multiplicative() {
        let mut pr = sample_pr(1, 0, 100);
        pr.labels = vec!["wip".to_string()];

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: Some(vec![LabelEffect {
                name: "wip".to_string(),
                effect: "x0.5".to_string(),
            }]),
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 50.0);
    }

    #[test]
    fn test_label_case_insensitive() {
        let mut pr = sample_pr(1, 0, 100);
        pr.labels = vec!["Urgent".to_string()]; // Capital U

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: Some(vec![
                LabelEffect {
                    name: "urgent".to_string(),
                    effect: "+10".to_string(),
                }, // lowercase
            ]),
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 110.0); // Should match despite case difference
    }

    #[test]
    fn test_multiple_labels_compound() {
        let mut pr = sample_pr(1, 0, 100);
        pr.labels = vec!["urgent".to_string(), "critical".to_string()];

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: Some(vec![
                LabelEffect {
                    name: "urgent".to_string(),
                    effect: "+10".to_string(),
                },
                LabelEffect {
                    name: "critical".to_string(),
                    effect: "x2".to_string(),
                },
            ]),
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        // (100 + 10) * 2 = 220
        assert_eq!(result.score, 220.0);
    }

    #[test]
    fn test_label_no_match() {
        let mut pr = sample_pr(1, 0, 100);
        pr.labels = vec!["bug".to_string()];

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: Some(vec![LabelEffect {
                name: "urgent".to_string(),
                effect: "+10".to_string(),
            }]),
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 100.0); // No label match, no effect
    }

    #[test]
    fn test_previously_reviewed_applies() {
        let mut pr = sample_pr(1, 0, 100);
        pr.user_has_reviewed = true;

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: None,
            previously_reviewed: Some("x0.5".to_string()),
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 50.0);
    }

    #[test]
    fn test_previously_reviewed_not_reviewed() {
        let mut pr = sample_pr(1, 0, 100);
        pr.user_has_reviewed = false;

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: None,
            previously_reviewed: Some("x0.5".to_string()),
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 100.0); // Not reviewed, effect not applied
    }

    #[test]
    fn test_size_uses_filtered_size() {
        let mut pr = sample_pr(1, 0, 1000); // additions=500, deletions=500, total=1000
        pr.filtered_size = Some(50); // After exclusion, only 50 lines

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: Some(SizeConfig {
                exclude: None,
                buckets: Some(vec![
                    SizeBucket {
                        range: "<100".to_string(),
                        effect: "x5".to_string(),
                    },
                    SizeBucket {
                        range: ">=100".to_string(),
                        effect: "x1".to_string(),
                    },
                ]),
            }),
            labels: None,
            previously_reviewed: None,
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        // filtered_size=50, matches <100 bucket -> x5, so 100 * 5 = 500
        assert_eq!(result.score, 500.0);
    }

    #[test]
    fn test_full_scoring_with_all_factors() {
        let mut pr = sample_pr(5, 2, 50); // 5h old, 2 approvals, 50 lines
        pr.labels = vec!["urgent".to_string()];
        pr.user_has_reviewed = false;

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: Some("+1 per 1h".to_string()),
            approvals: Some("+10 per 1".to_string()),
            size: Some(SizeConfig {
                exclude: None,
                buckets: Some(vec![SizeBucket {
                    range: "<100".to_string(),
                    effect: "x2".to_string(),
                }]),
            }),
            labels: Some(vec![LabelEffect {
                name: "urgent".to_string(),
                effect: "+20".to_string(),
            }]),
            previously_reviewed: Some("x0.5".to_string()),
            draft: None,
        };

        let result = calculate_score(&pr, &config);
        // (100 + 5 + 20) * 2 + 20 = 270
        // Wait, order: base=100, age=+5 (105), approvals=+20 (125), size=x2 (250), labels=+20 (270), previously_reviewed not applied (false)
        assert_eq!(result.score, 270.0);
    }

    #[test]
    fn test_draft_applies() {
        let mut pr = sample_pr(1, 0, 100);
        pr.draft = true;

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: None,
            previously_reviewed: None,
            draft: Some("x0.1".to_string()),
        };

        let result = calculate_score(&pr, &config);
        assert!((result.score - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_draft_not_applied_when_not_draft() {
        let mut pr = sample_pr(1, 0, 100);
        pr.draft = false;

        let config = ScoringConfig {
            base_score: Some(100.0),
            age: None,
            approvals: None,
            size: None,
            labels: None,
            previously_reviewed: None,
            draft: Some("x0.1".to_string()),
        };

        let result = calculate_score(&pr, &config);
        assert_eq!(result.score, 100.0);
    }
}
