use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub(crate) fn score(matcher: &SkimMatcherV2, haystack: &str, query: &str) -> Option<i64> {
    let haystack = haystack.to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .try_fold(0_i64, |score, term| {
            matcher
                .fuzzy_match(&haystack, term)
                .map(|term_score| score.saturating_add(term_score))
        })
}

pub(crate) fn indices(matcher: &SkimMatcherV2, haystack: &str, query: &str) -> Option<Vec<usize>> {
    let haystack = haystack.to_lowercase();
    let mut indices = Vec::new();
    for term in query.to_lowercase().split_whitespace() {
        let (_, term_indices) = matcher.fuzzy_indices(&haystack, term)?;
        indices.extend(term_indices);
    }
    indices.sort_unstable();
    indices.dedup();
    Some(indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_separated_terms_are_anded() {
        let matcher = SkimMatcherV2::default();

        assert!(score(&matcher, "node:@acme/api#test", "api test").is_some());
        assert!(score(&matcher, "node:@acme/api#test", "test api").is_some());
        assert!(score(&matcher, "node:@acme/api#lint", "api test").is_none());
    }

    #[test]
    fn repeated_whitespace_is_ignored() {
        let matcher = SkimMatcherV2::default();

        assert!(score(&matcher, "node:@acme/api#test", " api  test ").is_some());
        assert_eq!(score(&matcher, "anything", "  \t "), Some(0));
    }

    #[test]
    fn indices_include_matches_from_every_term() {
        let matcher = SkimMatcherV2::default();

        assert_eq!(
            indices(&matcher, "node:@acme/api#test", "api test"),
            Some(vec![11, 12, 13, 15, 16, 17, 18])
        );
    }
}
