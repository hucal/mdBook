use renderer::Renderer;
use book::MDBook;
use book::bookitem::{BookItem, Chapter};
use {utils, theme};
use theme::{Theme};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::Write;
use std::env;
use regex::Regex;
use errors::*;


#[derive(Default)]
pub struct PandocRenderer {
    format: String
}

impl PandocRenderer {
    pub fn new() -> Self {
        PandocRenderer { format: "html".into() }
    }

    pub fn set_format(&mut self, format: &str) {
        self.format = format.into();
    }

    fn render_book(&self, book: &MDBook, css_files : &Vec<PathBuf>, source_files : &Vec<PathBuf>) -> Result<()> {
        let target_base = book.get_source().canonicalize()?
            .parent().ok_or("Invalid bookitem path!")?
            .join("book"); // TODO right target path?
        let target_path = target_base.join(format!("book.{}", self.format));

        info!("[*] Rendering book to file {:?}", target_path);

        let mut cmd = Command::new("pandoc");

        for css in css_files {
            let css_path = target_base.join(target_base.join(css));
            cmd.arg(
                format!("--css={:?}", css_path));
        }

        env::set_current_dir(target_base)?;
        let output = cmd
            .args(&["--standalone", "--toc", "--number-sections", "--latex-engine=xelatex"])
            .args(source_files)
            .arg("-o").arg(target_path)
            .output()
            .expect("Could not run pandoc");

        if output.stdout.len() > 0 
        { info!("Pandoc stdout: {}", String::from_utf8(output.stdout)?); }

        if output.stderr.len() > 0
        { info!("Pandoc stderr: {}", String::from_utf8(output.stderr)?); }

        Ok(())
    }

    fn preprocess(&self, section_name : Vec<&str>, title : &str, source_path : &PathBuf, target_path : &PathBuf) -> Result<()> {
        let content = utils::fs::file_to_string(source_path)?;
        let mut file = utils::fs::create_file(target_path)?;

        // Do not process Affix BookItems, only Chapters
        if section_name.len() > 0 {
            let re = Regex::new(r"(?xm) 
              # enable insignificant whitespace mode and comments, enable multi-line mode
              ^
              (?P<t>\#+ .*)  # capture named group 't'
              $
            ").unwrap();

            // Increment the depth of all headings in the markdown
            let mut n = String::new(); 
            for _ in 0..section_name.len() {
                n.push('#');
            }

            let processed = re.replace_all(content.as_str(), format!("{}$t", n).as_str());

            // Write chapter title
            let result = format!("{} {}\n\n{}", n, title, processed.into_owned());
            file.write_all(result.as_str().as_bytes())?;
        } else {
            file.write_all(content.as_str().as_bytes())?;
        }

        Ok(())

    }

    fn render_item(&self, item: &BookItem, mut ctx: RenderItemContext, source_files: &mut Vec<PathBuf>)
        -> Result<()> {
        // FIXME: This should be made DRY-er and rely less on mutable state
        match *item {
            BookItem::Chapter(_, ref ch) |
            BookItem::Affix(ref ch) if !ch.path.as_os_str().is_empty() => {

                let mut section_name = match *item {
                    BookItem::Chapter(ref name, _) => { 
                        name.split(".").collect()
                    },
                    _ => { vec![] }
                };
                section_name.pop();


                let source_base = ctx.book.get_source().canonicalize()?;
                let source_path = source_base 
                    .join(&ch.path);
                let target_base = source_base
                    .parent().ok_or("Invalid bookitem path!")?
                    .join("tmp"); // TODO right target path?
                let target_path = target_base.join(&ch.path);
                // TODO utils::fs::remove_dir_content(".../tmp")


                // Parse and expand links
                // TODO render markdown
                //let content = preprocess::links::replace_all(&content, base)?;
                //let content = utils::render_markdown(&content, ctx.book.get_curly_quotes());
                //print_content.push_str(&content);


                let chapter_title = &ch.name;
                //ctx.data.insert("path".to_owned(), json!(path));
                //ctx.data.insert("content".to_owned(), json!(content));
                //ctx.data.insert("chapter_title".to_owned(), json!(ch.name));
                //ctx.data.insert(
                //    "path_to_root".to_owned(),
                //    json!(utils::fs::path_to_root(&ch.path)),
                //);

                self.preprocess(section_name, chapter_title, &source_path, &target_path)?;


                // Write to file
                info!("[*] Creating {:?} ✓", target_path.display());
                //ctx.book.write_file(filename, &rendered.into_bytes())?;

                source_files.push(target_path);

                //if ctx.is_index {
                //    self.render_index(ctx.book, ch, &ctx.destination)?;
                //}

            },
            _ => {},
        }
        Ok(())
    }


    //fn post_process(&self, rendered: String, filename: &str, playpen_config: &PlaypenConfig) -> String {
    //    let rendered = build_header_links(&rendered, filename);
    //    let rendered = fix_anchor_links(&rendered, filename);
    //    let rendered = fix_code_blocks(&rendered);
    //    let rendered = add_playpen_pre(&rendered, playpen_config);

    //    rendered
    //}

    fn copy_static_files(&self, book: &MDBook, theme: &Theme, css_files: &mut Vec<PathBuf>) -> Result<()> {
        css_files.push("book.css".into());
        css_files.push("highlight.css".into());
        css_files.push("tomorrow-night.css".into());
        css_files.push("ayu-highlight.css".into());

        book.write_file("book.css", &theme.css)?;
        book.write_file("favicon.png", &theme.favicon)?;
        book.write_file("highlight.css", &theme.highlight_css)?;
        book.write_file(
            "tomorrow-night.css",
            &theme.tomorrow_night_css,
        )?;
        book.write_file(
            "ayu-highlight.css",
            &theme.ayu_highlight_css,
        )?;

        Ok(())
    }

    /// Helper function to write a file to the build directory, normalizing 
    /// the path to be relative to the book root.
    fn write_custom_file(&self, custom_file: &Path, book: &MDBook) -> Result<()> {
        let mut data = Vec::new();
        let mut f = File::open(custom_file)?;
        f.read_to_end(&mut data)?;

        let name = match custom_file.strip_prefix(book.get_root()) {
            Ok(p) => p.to_str().expect("Could not convert to str"),
            Err(_) => {
                custom_file
                    .file_name()
                    .expect("File has a file name")
                    .to_str()
                    .expect("Could not convert to str")
            },
        };

        book.write_file(name, &data)?;

        Ok(())
    }


    fn copy_additional_css_and_js(&self, book: &MDBook, css_files: &mut Vec<PathBuf>) -> Result<()> {
        for custom_css in book.get_additional_css().iter() {
            css_files.push(custom_css.clone());
            self.write_custom_file(custom_css, book)?;
        }
            
        for custom_file in book.get_additional_js().iter() {
            self.write_custom_file(custom_file, book)?;
        }

        Ok(())
    }
}


impl Renderer for PandocRenderer {
    fn render(&self, book: &MDBook) -> Result<()> {
        // debug!("[fn]: render");
        // let mut handlebars = Handlebars::new();

        let theme = theme::Theme::new(book.get_theme_path());


        //let mut data = make_data(book)?;

        // Print version
        let mut print_content = String::new();

        let destination = book.get_destination();

        debug!("[*]: Check if destination directory exists");
        if fs::create_dir_all(&destination).is_err() {
            bail!("Unexpected error when constructing destination path");
        }

        let mut source_files = vec![];

        for (i, item) in book.iter().enumerate() {
            let ctx = RenderItemContext {
                book: book,
                destination: destination.to_path_buf(),
                //data: data.clone(),
                is_index: i == 0,
            };
            self.render_item(item, ctx, &mut source_files)?;
        }


        // Copy static files (js, css, images, ...)
        debug!("[*] Copy static files");
        let mut css_files = vec![];
        self.copy_static_files(book, &theme, &mut css_files)?;
        self.copy_additional_css_and_js(book, &mut css_files)?;

        // Copy all remaining files
        utils::fs::copy_files_except_ext(book.get_source(), destination, true, &["md"])?;

        self.render_book(book, &css_files, &source_files)?;

        // Print version
        //self.configure_print_version(&mut data, &print_content);

        // Render the handlebars template with the data
        //debug!("[*]: Render template");

        //let rendered = handlebars.render("index", &data)?;

        //let rendered = self.post_process(rendered, "print.html",
        //    book.get_html_config().get_playpen_config());
        
        //book.write_file(
        //    Path::new("print").with_extension("html"),
        //    &rendered.into_bytes(),
        //)?;
        //info!("[*] Creating print.html ✓");

        Ok(())
    }
}

fn make_data(book: &MDBook) { // -> Result<serde_json::Map<String, serde_json::Value>> {

    //data.insert("language".to_owned(), json!("en"));
    //data.insert("title".to_owned(), json!(book.get_title()));
    //data.insert("description".to_owned(), json!(book.get_description()));
    //data.insert("favicon".to_owned(), json!("favicon.png"));

    // Add check to see if there is an additional style
    if book.has_additional_css() {
        let mut css = Vec::new();
        for style in book.get_additional_css() {
            match style.strip_prefix(book.get_root()) {
                Ok(p) => css.push(p.to_str().expect("Could not convert to str")),
                Err(_) => {
                    css.push(
                        style
                            .file_name()
                            .expect("File has a file name")
                            .to_str()
                            .expect("Could not convert to str"),
                    )
                },
            }
        }
        //data.insert("additional_css".to_owned(), json!(css));
    }

    // Add check to see if there is an additional script
    if book.has_additional_js() {
        // TODO ignore?
        let mut js = Vec::new();
        for script in book.get_additional_js() {
            match script.strip_prefix(book.get_root()) {
                Ok(p) => js.push(p.to_str().expect("Could not convert to str")),
                Err(_) => {
                    js.push(
                        script
                            .file_name()
                            .expect("File has a file name")
                            .to_str()
                            .expect("Could not convert to str"),
                    )
                },
            }
        }
    }


    //let mut chapters = vec![];

    for item in book.iter() {
        // Create the data to inject in the template

        match *item {
            BookItem::Affix(ref ch) => {
                //chapter.insert("name".to_owned(), json!(ch.name));
                //let path = ch.path.to_str().ok_or_else(|| {
                //    io::Error::new(io::ErrorKind::Other, "Could not convert path to str")
                //})?;
                //chapter.insert("path".to_owned(), json!(path));
            },
            BookItem::Chapter(ref s, ref ch) => {
                //chapter.insert("section".to_owned(), json!(s));
                //chapter.insert("name".to_owned(), json!(ch.name));
                //let path = ch.path.to_str().ok_or_else(|| {
                //    io::Error::new(io::ErrorKind::Other, "Could not convert path to str")
                //})?;
                //chapter.insert("path".to_owned(), json!(path));
            },
            BookItem::Spacer => {
                //chapter.insert("spacer".to_owned(), json!("_spacer_"));
            },

        }

        //chapters.push(chapter);
    }

    //data.insert("chapters".to_owned(), json!(chapters));

    //Ok(data)
}

// Goes through the rendered HTML, making sure all header tags are wrapped in
// an anchor so people can link to sections directly.
// fn build_header_links(html: &str, filename: &str) -> String {

// Wraps a single header tag with a link, making sure each tag gets its own
// unique ID by appending an auto-incremented number (if necessary).
//fn wrap_header_with_link(level: usize, content: &str, id_counter: &mut HashMap<String, usize>, filename: &str)

// Generate an id for use with anchors which is derived from a "normalised"
// string.
//fn id_from_content(content: &str) -> String {

// anchors to the same page (href="#anchor") do not work because of
// <base href="../"> pointing to the root folder. This function *fixes*
// that in a very inelegant way
// fn fix_anchor_links(html: &str, filename: &str) -> String {


// The rust book uses annotations for rustdoc to test code snippets,
// like the following:
// ```rust,should_panic
// fn main() {
//     // Code here
// }
// ```
// This function replaces all commas by spaces in the code block classes
//fn fix_code_blocks(html: &str) -> String {
//    let regex = Regex::new(r##"<code([^>]+)class="([^"]+)"([^>]*)>"##).unwrap();
//    regex
//        .replace_all(html, |caps: &Captures| {
//            let before = &caps[1];
//            let classes = &caps[2].replace(",", " ");
//            let after = &caps[3];
//
//            format!(r#"<code{before}class="{classes}"{after}>"#, before = before, classes = classes, after = after)
//        })
//        .into_owned()
//}

//fn add_playpen_pre(html: &str, playpen_config: &PlaypenConfig) -> String {
//    let regex = Regex::new(r##"((?s)<code[^>]?class="([^"]+)".*?>(.*?)</code>)"##).unwrap();
//    regex
//        .replace_all(html, |caps: &Captures| {
//            let text = &caps[1];
//            let classes = &caps[2];
//            let code = &caps[3];
//
//            if classes.contains("language-rust") && !classes.contains("ignore") {
//                // wrap the contents in an external pre block
//                if playpen_config.is_editable() &&
//                    classes.contains("editable") || text.contains("fn main") || text.contains("quick_main!") {
//                    format!("<pre class=\"playpen\">{}</pre>", text)
//                } else {
//                    // we need to inject our own main
//                    let (attrs, code) = partition_source(code);
//
//                    format!("<pre class=\"playpen\"><code class=\"{}\">\n# #![allow(unused_variables)]\n\
//                        {}#fn main() {{\n\
//                        {}\
//                        #}}</code></pre>",
//                        classes, attrs, code)
//                }
//            } else {
//                // not language-rust, so no-op
//                text.to_owned()
//            }
//        })
//        .into_owned()
//}

//fn partition_source(s: &str) -> (String, String) {
//    let mut after_header = false;
//    let mut before = String::new();
//    let mut after = String::new();
//
//    for line in s.lines() {
//        let trimline = line.trim();
//        let header = trimline.chars().all(|c| c.is_whitespace()) || trimline.starts_with("#![");
//        if !header || after_header {
//            after_header = true;
//            after.push_str(line);
//            after.push_str("\n");
//        } else {
//            before.push_str(line);
//            before.push_str("\n");
//        }
//    }
//
//    (before, after)
//}


struct RenderItemContext<'a> {
    book: &'a MDBook,
    destination: PathBuf,
    //data: serde_json 
    is_index: bool,
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_build_header_links() {
        let inputs = vec![
            // ("blah blah <h1>Foo</h1>", r#"blah blah <a class="header" href="bar.rs#foo" id="foo"><h1>Foo</h1></a>"#)
        ];

        for (src, should_be) in inputs {
            let got = build_header_links(src, "bar.rs");
            assert_eq!(got, should_be);
        }
    }
}
