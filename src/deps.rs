use std::{
    borrow::Cow,
    collections::{HashSet, VecDeque},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

const SOURCE_EXTS: [&str; 4] = ["cpp", "cc", "cxx", "c++"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeKind {
    System,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Include {
    pub path: String,
    pub kind: IncludeKind,
}

#[derive(Debug, Default)]
pub struct Sources {
    pub units: Vec<PathBuf>,
    pub headers: Vec<PathBuf>,
    pub system: Vec<String>,
    pub unresolved: Vec<(PathBuf, String)>,
}

pub fn parse_includes(src: &str) -> Vec<Include> {
    // Line splices are rare, so only copy the file when there is one.
    let spliced = if src.contains("\\\n") || src.contains("\\\r\n") {
        Cow::Owned(src.replace("\\\r\n", "").replace("\\\n", ""))
    } else {
        Cow::Borrowed(src)
    };
    strip_comments(&spliced)
        .lines()
        .filter_map(parse_directive)
        .collect()
}

fn parse_directive(line: &str) -> Option<Include> {
    let rest = line
        .trim_start()
        .strip_prefix('#')?
        .trim_start()
        .strip_prefix("include")?;

    if !rest.starts_with(|c: char| c.is_whitespace() || c == '<' || c == '"') {
        return None;
    }

    let rest = rest.trim_start();
    let (kind, close, body) = if let Some(body) = rest.strip_prefix('<') {
        (IncludeKind::System, '>', body)
    } else {
        (IncludeKind::Local, '"', rest.strip_prefix('"')?)
    };

    let path = &body[..body.find(close)?];
    (!path.is_empty()).then(|| Include {
        path: path.to_string(),
        kind,
    })
}

fn strip_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;

    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                i = find_from(b, i, b"\n").unwrap_or(b.len());
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let end = find_from(b, i + 2, b"*/").map_or(b.len(), |e| e + 2);
                out.push(b' ');
                out.extend(b[i..end].iter().filter(|&&c| c == b'\n'));
                i = end;
            }
            b'"' if is_raw_string_prefix(&out) => {
                let open = find_from(b, i, b"(").unwrap_or(b.len());
                let mut terminator = b")".to_vec();
                terminator.extend_from_slice(&b[i + 1..open]);
                terminator.push(b'"');
                let end = find_from(b, open, &terminator).map_or(b.len(), |e| e + terminator.len());
                out.extend_from_slice(b"\"\"");
                out.extend(b[i..end].iter().filter(|&&c| c == b'\n'));
                i = end;
            }
            b'"' => i = copy_literal(b, i, b'"', &mut out),
            b'\'' if !ends_with_alnum(&out) || is_char_prefix(&out) => {
                i = copy_literal(b, i, b'\'', &mut out);
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }

    // Reuses `out`'s buffer; only invalid UTF-8 in the source costs a copy.
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

fn copy_literal(b: &[u8], start: usize, quote: u8, out: &mut Vec<u8>) -> usize {
    out.push(quote);
    let mut i = start + 1;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            return i;
        }
        out.push(c);
        i += 1;
        if c == b'\\' && i < b.len() && b[i] != b'\n' {
            out.push(b[i]);
            i += 1;
        } else if c == quote {
            break;
        }
    }
    i
}

fn find_from(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn trailing_ident(out: &[u8]) -> &[u8] {
    let start = out
        .iter()
        .rposition(|&c| !(c.is_ascii_alphanumeric() || c == b'_'))
        .map_or(0, |p| p + 1);
    &out[start..]
}

fn is_raw_string_prefix(out: &[u8]) -> bool {
    matches!(trailing_ident(out), b"R" | b"u8R" | b"uR" | b"UR" | b"LR")
}

fn is_char_prefix(out: &[u8]) -> bool {
    matches!(trailing_ident(out), b"u8" | b"u" | b"U" | b"L")
}

fn ends_with_alnum(out: &[u8]) -> bool {
    out.last().is_some_and(|c| c.is_ascii_alphanumeric())
}

pub fn collect(entry: &Path, include_dirs: &[&Path], src_dirs: &[&Path]) -> io::Result<Sources> {
    let mut sources = Sources::default();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<(PathBuf, bool)> = VecDeque::new();

    let entry = entry.to_path_buf();
    seen.insert(fs::canonicalize(&entry)?);
    queue.push_back((entry, true));

    while let Some((file, is_unit)) = queue.pop_front() {
        let content = fs::read_to_string(&file)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", file.display())))?;

        let dir = file.parent().unwrap_or(Path::new(""));
        for inc in parse_includes(&content) {
            let local_dir = (inc.kind == IncludeKind::Local).then_some(dir);
            let Some(header) = local_dir
                .into_iter()
                .chain(include_dirs.iter().copied())
                .map(|d| d.join(&inc.path))
                .find(|p| p.is_file())
            else {
                match inc.kind {
                    // A linear scan beats a HashSet<String> copy here: there are only a few dozen.
                    IncludeKind::System => {
                        if !sources.system.contains(&inc.path) {
                            sources.system.push(inc.path);
                        }
                    }
                    IncludeKind::Local => sources.unresolved.push((file.clone(), inc.path)),
                }
                continue;
            };

            if seen.insert(fs::canonicalize(&header)?) {
                if let Some(imp) = find_implementation(&header, include_dirs, src_dirs)
                    && seen.insert(fs::canonicalize(&imp)?)
                {
                    queue.push_back((imp, true));
                }
                queue.push_back((header, false));
            }
        }

        if is_unit {
            sources.units.push(file);
        } else {
            sources.headers.push(file);
        }
    }

    Ok(sources)
}

fn find_implementation(header: &Path, include_dirs: &[&Path], src_dirs: &[&Path]) -> Option<PathBuf> {
    let stem = header.file_stem()?;
    let header_dir = header.parent().unwrap_or(Path::new(""));
    let sub = include_dirs.iter().find_map(|inc| header_dir.strip_prefix(inc).ok());

    // header dir, then for each src dir: src/<sub>, src
    let dirs = std::iter::once(Cow::Borrowed(header_dir)).chain(src_dirs.iter().flat_map(|&src| {
        sub.map(|sub| Cow::Owned(src.join(sub)))
            .into_iter()
            .chain(std::iter::once(Cow::Borrowed(src)))
    }));

    // Build the file name once per extension. Not `with_extension`: that would
    // turn `foo.test.h` into `foo.cpp` instead of `foo.test.cpp`.
    let mut name = OsString::with_capacity(stem.len() + 4);
    for dir in dirs {
        for ext in SOURCE_EXTS {
            name.clear();
            name.push(stem);
            name.push(".");
            name.push(ext);
            let path = dir.join(&name);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(p: &str) -> Include {
        Include {
            path: p.into(),
            kind: IncludeKind::Local,
        }
    }
    fn system(p: &str) -> Include {
        Include {
            path: p.into(),
            kind: IncludeKind::System,
        }
    }

    #[test]
    fn basic() {
        let src = "#include <iostream>\n#include \"user.h\"\nint main() {}\n";
        assert_eq!(
            parse_includes(src),
            vec![system("iostream"), local("user.h")]
        );
    }

    #[test]
    fn whitespace_variants() {
        let src = "  #include <a>\n#  include \"b.h\"\n\t#\tinclude<c>\n#include\"d.h\"\n#include <e>   \r\n";
        assert_eq!(
            parse_includes(src),
            vec![
                system("a"),
                local("b.h"),
                system("c"),
                local("d.h"),
                system("e")
            ]
        );
    }

    #[test]
    fn trailing_comments_and_non_ascii() {
        let src =
            "#include \"a.h\" // see \"b.h\"\n#include <c> /* \"d\" */\n#include \"e.h\" // café\n";
        assert_eq!(
            parse_includes(src),
            vec![local("a.h"), system("c"), local("e.h")]
        );
    }

    #[test]
    fn commented_out() {
        let src = "// #include \"a.h\"\n/*\n#include \"b.h\"\n*/\n/* x */ #include \"c.h\"\n";
        assert_eq!(parse_includes(src), vec![local("c.h")]);
    }

    #[test]
    fn not_includes() {
        let src = "#include_next <a>\n#define X \"#include <b>\"\n#include MACRO\n#include <>\nint x; // #include <c>\n";
        assert_eq!(parse_includes(src), vec![]);
    }

    #[test]
    fn comment_markers_inside_strings() {
        let src =
            "const char* s = \"/* not a comment\";\n#include \"a.h\"\nconst char* t = \"*/\";\n";
        assert_eq!(parse_includes(src), vec![local("a.h")]);
    }

    #[test]
    fn raw_strings() {
        let src = "auto s = R\"x(\n#include \"fake.h\"\n)\" still raw )x\";\n#include \"real.h\"\n";
        assert_eq!(parse_includes(src), vec![local("real.h")]);
    }

    #[test]
    fn digit_separators_and_char_literals() {
        let src = "int n = 1'000'000; /* start\n#include \"fake.h\"\n*/\nchar q = '\"';\n#include \"real.h\"\n";
        assert_eq!(parse_includes(src), vec![local("real.h")]);
    }

    #[test]
    fn line_splice() {
        assert_eq!(
            parse_includes("#include \\\n  \"a.h\"\n"),
            vec![local("a.h")]
        );
    }

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(name: &str) -> Self {
            let p = std::env::temp_dir().join(format!("cono-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        fn write(&self, rel: &str, content: &str) {
            let p = self.0.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn dotted_header_name_finds_matching_source() {
        let t = TempDir::new("dotted");
        t.write("main.cpp", "#include \"foo.test.h\"\n");
        t.write("foo.test.h", "");
        t.write("foo.test.cpp", "");
        t.write("foo.cpp", ""); // the old code wrongly picked this one
        let r = &t.0;
        let s = collect(&r.join("main.cpp"), &[], &[]).unwrap();
        assert_eq!(s.units, vec![r.join("main.cpp"), r.join("foo.test.cpp")]);
    }

    #[test]
    fn collects_transitive_sources() {
        let t = TempDir::new("collect");
        t.write("main.cpp", "#include <iostream>\n#include \"util.h\"\n#include <net/socket.h>\n#include \"missing.h\"\n");
        t.write("util.h", "#pragma once\n#include \"util.h\"\n"); // self-include: cycle
        t.write("util.cpp", "#include \"util.h\"\n#include \"detail.h\"\n");
        t.write(
            "detail.h",
            "#pragma once\n#include <vector>\n#include <iostream>\n",
        ); // header only
        t.write(
            "include/net/socket.h",
            "#pragma once\n#include \"util.h\"\n",
        );
        t.write("src/net/socket.cpp", "#include <net/socket.h>\n");

        let r = &t.0;
        let s = collect(
            &r.join("main.cpp"),
            &[&r.join("include"), r],
            &[&r.join("src")],
        )
        .unwrap();

        assert_eq!(
            s.units,
            vec![
                r.join("main.cpp"),
                r.join("util.cpp"),
                r.join("src/net/socket.cpp")
            ]
        );
        assert_eq!(
            s.headers,
            vec![
                r.join("util.h"),
                r.join("include/net/socket.h"),
                r.join("detail.h")
            ]
        );
        assert_eq!(s.system, vec!["iostream", "vector"]);
        assert_eq!(
            s.unresolved,
            vec![(r.join("main.cpp"), "missing.h".to_string())]
        );
    }
}
