//! Lesson 5: the whisper.cpp shell pipeline. The pipeline itself is `scripts/whisper-pipeline.sh`;
//! this is the same contract in Rust, so the shell script's output can be checked and falsified here.

/// The merged transcript holds one non-empty line per input file.
pub fn merged_contract_holds(merged: &str, files: usize) -> bool {
    let lines: Vec<&str> = merged.lines().collect();
    lines.len() == files && lines.iter().all(|l| !l.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_for_one_line_per_file() {
        assert!(merged_contract_holds("a b\nc d\n", 2));
    }

    #[test]
    fn fails_on_an_empty_line() {
        assert!(!merged_contract_holds("a\n\n", 2));
    }

    #[test]
    fn fails_on_a_whitespace_only_line() {
        assert!(!merged_contract_holds("a\n   \n", 2));
    }

    #[test]
    fn fails_when_a_file_is_missing_from_the_merge() {
        assert!(!merged_contract_holds("a\n", 2));
    }

    #[test]
    fn fails_when_there_is_an_extra_line() {
        assert!(!merged_contract_holds("a\nb\n", 1));
    }

    #[test]
    fn fails_for_an_empty_merge() {
        assert!(!merged_contract_holds("", 1));
    }
}
