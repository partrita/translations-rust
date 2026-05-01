use mdbook_driver::book::{Book, BookItem};
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};
use anyhow::Result;
use std::path::Path;
use std::collections::HashMap;

/// A preprocessor mdBook, so as to treat the data flow to translate, when
/// necessary, items contained inside code blocks.
/// Translation is done on a per-line exact match basis.
pub struct CodeTranslator {
    translations: HashMap<String, String>,
}

impl CodeTranslator{
    pub fn new(po_file: &Path) -> Result<Self> {
        let catalog = polib::po_file::parse(po_file)?;

        let mut translations = HashMap::new();

        for message in catalog.messages() {
            if let Ok(msgstr) = message.msgstr() {
                translations.insert(
                    message.msgid().to_string(),
                    msgstr.to_string(),
                );
            }
        }
        Ok(Self {translations})
    }

    fn translate_line(&self, line: &str) -> String {
        // Case 1: exact match lookup:
        if let Some(translated) = self.translations.get(line) {
            return translated.clone();
        }

        // Case 2: hidden Rust code (lines starting with "# "):
        if let Some(stripped) = line.strip_prefix("# ") {
            if let Some(translated) = self.translations.get(stripped) {
                return format!("# {}", translated);
            }
        }

        line.to_string()
    }
}

impl Preprocessor for CodeTranslator {
    fn name(&self) -> &str {
        "code-translator"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        for item in &mut book.items {
            if let BookItem::Chapter(chapter) = item {
                chapter.content = crate::listings::process_code_blocks(
                    &chapter.content,
                    |line| {
                        self.translate_line(line)
                });
                process_item(item, &|line| self.translate_line(line));
            };
        }
        Ok(book)
    }
}

fn process_item<F>(item: &mut BookItem, f: &F)
where
    F: Fn(&str) -> String,
{
    if let BookItem::Chapter(chapter) = item {
        chapter.content = crate::listings::process_code_blocks(
            &chapter.content,
            |line| f(line),
        );

        for sub in &mut chapter.sub_items {

            process_item(sub, f);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_translation_map_from_catalog() {
        let mut translation_map = HashMap::new();
        translation_map.insert("Hello".to_string(), "Salut".to_string());
        translation_map.insert("World".to_string(), "Monde".to_string());

        let translator = CodeTranslator { translations: translation_map };

        assert_eq!(translator.translate_line("Hello"), "Salut");
        assert_eq!(translator.translate_line("World"), "Monde");
    }

    #[test]
    fn returns_original_line_if_translation_not_found() {
        let translator = CodeTranslator { translations: HashMap::new() };

        assert_eq!(translator.translate_line("Whatever unknown line"),
                                             "Whatever unknown line");
    }

    #[test]
    fn translates_code_block_via_closure() {
        let mut translation_map = HashMap::new();
        translation_map.insert("hello".to_string(), "salut".to_string());

        let translator = CodeTranslator{ translations: translation_map};

        let input = "```\nhello\n```";

        let output = crate::listings::process_code_blocks(
            input,
            |line| translator.translate_line(line),
        );

        assert_eq!(output, "```\nsalut\n```\n");
    }

    #[test]
    fn translates_hidden_code_lines() {
        let mut translation_map = HashMap::new();
        translation_map.insert("let x = 5;".to_string(), "let x = cinq;".to_string());

        let translator = super::CodeTranslator { translations: translation_map };

        let input = "```\n# let x = 5;\n```";

        let output = crate::listings::process_code_blocks(
            input,
            |line| translator.translate_line(line),
        );

        assert_eq!(output, "```\n# let x = cinq;\n```\n");
    }
    #[test]
    fn translates_code_in_subchapter() {
        use mdbook_driver::book::{Book, BookItem, Chapter};
        let subchapter_content = r#"
```rust
const TMP: u32 = 32;
```
"#;

        let subchapter = Chapter::new(
            "Sub",
            subchapter_content.to_string(),
            "sub.md",
            Vec::new(),
        );

        let mut chapter = Chapter::new(
            "Main",
            "".to_string(),
            "main.md",
            Vec::new(),
        );

        chapter.sub_items.push(BookItem::Chapter(subchapter));


        let mut book = Book::new();
        book.items.push(BookItem::Chapter(chapter));

        let translator = |line: &str| {
            if line.contains("const TMP") {
                "const TEMP: u32 = 64;".to_string()
            } else {
                line.to_string()
            }
        };

        for item in &mut book.items {
            process_item(item, &translator);
        }

        if let BookItem::Chapter(ch) = &book.items[0] {
            if let BookItem::Chapter(sub) = &ch.sub_items[0] {
                assert!(sub.content.contains("const TEMP: u32 = 64"));
            } else {
                panic!("Expected subchapter");
            }
        } else {
            panic!("Expected chapter");
        }
    }
}
