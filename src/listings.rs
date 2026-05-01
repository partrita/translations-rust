/// Process Markdown input and apply a transformation function only
/// inside fenced code blocks (```).
/// This addresses the case where portions of code are also translated: not just
/// strings, but also variable names, structs, etc.; called before mdbook.build():
///
/// Design choices:
/// - We use a simple state machine (in_code: bool)
/// - We do NOT parse full Markdown (intentionally lightweight)
/// - Fence detection is naive (`starts_with("```")`)
///
/// Limitations:
/// - Does not support nested fences
/// - Assumes fences are well-formed
/// - Language tags after ``` are ignored
///
/// We deliberately keep code block parsing independent from mdBook
/// so it can be unit-tested without the mdBook runtime.
pub fn process_code_blocks<F>(input: &str, mut translate: F) -> String
where
    F: FnMut(&str) -> String,
{
    let mut output = String::new();
    let mut in_code = false;

    for line in input.lines() {
        // Toggle state when encountering a fence.
        // This assumes well-formed Markdown (paired ```).
        if line.starts_with("```") {
            in_code = !in_code;
            output.push_str(line);
            output.push('\n');
            continue;
        }
        if in_code {
            output.push_str(&translate(line));
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_modify_non_code() {
        // Tests that out of blocks of code delimited by ```, nothing changes
        let input = "Hello\nWorld!";
        let output = process_code_blocks(input, |line| {
            format!("XXX {line}")
        });

        assert_eq!(output, "Hello\nWorld!\n");
    }

    #[test]
    fn translates_lines_inside_code_blocks() {
        let input = "```\nHello\nWorld!\n```";

        let output = process_code_blocks(input, |line| {
            match line {
                "Hello" => "Bonjour".to_string(),
                "World!" => "Monde!".to_string(),
                _       => line.to_string(),
            }
        });

    assert_eq!(output, "```\nBonjour\nMonde!\n```\n");
    }

    #[test]
    fn handles_empty_code_blocks() {
        let input = "```\n```";

        let output = process_code_blocks(input, |line| line.to_string());

        assert_eq!(output, "```\n```\n");
    }

    #[test]
    fn handles_no_trailing_newline() {
        let input = "```\nhello\n```";

        let output = process_code_blocks(input, |line| line.to_string());

        assert!(output.ends_with("```\n"));
    }
}

