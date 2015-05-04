// NOTE converted from assets/kate-syntax/rust.xml

use gfx::Color;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Style {
    pub color: Color,
    pub bold: bool
}

// extensions=r".*\.rs" mimetype="text/x-rust"
pub struct Rust<'a, I> {
    lines: I,
    depth: usize,
    current_line: &'a str,
    output: Vec<(usize, Vec<(usize, Style)>)>
}

impl<'a, I> Rust<'a, I> where I: Iterator<Item=&'a str> {
    pub fn run(mut lines: I) -> (usize, Vec<(usize, Vec<(usize, Style)>)>) {
        let mut hl = Rust {
            current_line: lines.next().unwrap_or(""),
            lines: lines,
            depth: 0,
            output: vec![(0, vec![])]
        };
        hl.normal();
        (hl.depth, hl.output)
    }

    fn advance(&mut self, len: usize, style: Style) {
        self.current_line = &self.current_line[len..];

        let last = &mut self.output.last_mut().unwrap().1;
        if let Some(&mut (ref mut prev_len, prev_style)) = last.last_mut() {
            if style == prev_style {
                *prev_len += len;
                return;
            }
        }
        last.push((len, style))
    }

    fn next_line(&mut self) -> bool {
        if let Some(line) = self.lines.next() {
            self.current_line = line;
            self.output.push((self.depth, vec![]));
            true
        } else {
            self.current_line = "";
            false
        }
    }
}

macro_rules! rust_ident { () => ("[a-zA-Z_][a-zA-Z_0-9]*") }
macro_rules! rust_int_suf { () => ("([iu](8|16|32|64)?)?") }

macro_rules! re {
    ($($r:expr),+) => (regex!(concat!("^(?:", $($r,)* ")")))
}
macro_rules! kw {
    ($($r:expr),+) => (regex!(concat!("^(?:" $(,$r,)"|"* r")\b")))
}

macro_rules! goto {
    ($s:expr, {}) => (());
    ($s:expr, return) => ({ $s.depth -= 1; return;});
    ($s:expr, $cx:ident) => ({ $s.depth += 1; $s.$cx() })
}

#[cfg(not(test))]
macro_rules! cases {
    ($s:expr, $ds:ident:) => { {
        let ch_len = {
            let mut idx = $s.current_line.char_indices().map(|(i, _)| i);
            idx.next();
            idx.next().unwrap_or($s.current_line.len())
        };
        $s.advance(ch_len, styles::$ds)
    } };
    ($s:expr, $ds:ident: $re:expr; $($rest:tt)*) => { cases!($s, $ds: $re, $ds => {}; $($rest)*) };
    ($s:expr, $ds:ident: $re:expr => $goto:tt; $($rest:tt)*) => { cases!($s, $ds: $re, $ds => $goto; $($rest)*) };
    ($s:expr, $ds:ident: $re:expr, $style:ident; $($rest:tt)*) => { cases!($s, $ds: $re, $style => {}; $($rest)*) };
    ($s:expr, $ds:ident: $re:expr, $style:ident => $goto:tt; $($rest:tt)*) => {
        if let Some((0, len)) = $re.find($s.current_line) {
            $s.advance(len, styles::$style);
            goto!($s, $goto);
        } else {
            cases!($s, $ds: $($rest)*)
        }
    }
}

#[cfg(test)]
macro_rules! cases { ($($rest:tt)*) => {()} }

macro_rules! context {
    ($name:ident, $ds:ident: $($rest:tt)*) => { context!{$name, $ds => {}: $($rest)*} };
    ($name:ident, $ds:ident => $line_end_cx:tt: $($rest:tt)*) => {
        fn $name(&mut self) {
            loop {
                while !self.current_line.is_empty() {
                    cases!(self, $ds: $($rest)*);
                }
                if self.next_line() {
                    goto!(self, $line_end_cx);
                } else {
                    return;
                }
            }
        }
    }
}

macro_rules! normal_like {
    ($($x:tt)+) => {context!{$($x)*
        re!(r"\s+");
        kw!("fn"), Keyword => function;
        kw!("type"), Keyword => type_;

        kw!("abstract",
            "alignof",
            "become",
            "do",
            "final",
            "offsetof",
            "override",
            "priv",
            "pure",
            "sizeof",
            "typeof",
            "unsized",
            "yield"), Keyword;
         kw!("as",
            "box",
            "break",
            "const",
            "continue",
            "crate",
            "else",
            "enum",
            "extern",
            "for",
            "if",
            "impl",
            "in",
            "let",
            "loop",
            "match",
            "mod",
            "move",
            "mut",
            "pub",
            "ref",
            "return",
            "static",
            "struct",
            "super",
            "trait",
            "unsafe",
            "use",
            "virtual",
            "where",
            "while"), Keyword;
        kw!("bool",
            "int",
            "isize",
            "uint",
            "usize",
            "i8",
            "i16",
            "i32",
            "i64",
            "u8",
            "u16",
            "u32",
            "u64",
            "f32",
            "f64",
            "float",
            "char",
            "str",
            "Option",
            "Result",
            "Self",
            "Box",
            "Vec",
            "String"), Type;
        kw!("AsSlice",
            "CharExt",
            "Clone",
            "Copy",
            "Debug",
            "Decodable",
            "Default",
            "Display",
            "DoubleEndedIterator",
            "Drop",
            "Encodable",
            "Eq",
            "Default",
            "Extend",
            "Fn",
            "FnMut",
            "FnOnce",
            "FromPrimitive",
            "Hash",
            "Iterator",
            "IteratorExt",
            "MutPtrExt",
            "Ord",
            "PartialEq",
            "PartialOrd",
            "PtrExt",
            "Rand",
            "Send",
            "Sized",
            "SliceConcatExt",
            "SliceExt",
            "Str",
            "StrExt",
            "Sync",
            "ToString"), Trait;
        kw!("c_float",
            "c_double",
            "c_void",
            "FILE",
            "fpos_t",
            "DIR",
            "dirent",
            "c_char",
            "c_schar",
            "c_uchar",
            "c_short",
            "c_ushort",
            "c_int",
            "c_uint",
            "c_long",
            "c_ulong",
            "size_t",
            "ptrdiff_t",
            "clock_t",
            "time_t",
            "c_longlong",
            "c_ulonglong",
            "intptr_t",
            "uintptr_t",
            "off_t",
            "dev_t",
            "ino_t",
            "pid_t",
            "mode_t",
            "ssize_t"), CType;
        kw!("self"), SelfKw;
        kw!("true",
            "false",
            "Some",
            "None",
            "Ok",
            "Err",
            "Success",
            "Failure",
            "Cons",
            "Nil"), Constant;
        kw!("EXIT_FAILURE",
            "EXIT_SUCCESS",
            "RAND_MAX",
            "EOF",
            "SEEK_SET",
            "SEEK_CUR",
            "SEEK_END",
            "_IOFBF",
            "_IONBF",
            "_IOLBF",
            "BUFSIZ",
            "FOPEN_MAX",
            "FILENAME_MAX",
            "L_tmpnam",
            "TMP_MAX",
            "O_RDONLY",
            "O_WRONLY",
            "O_RDWR",
            "O_APPEND",
            "O_CREAT",
            "O_EXCL",
            "O_TRUNC",
            "S_IFIFO",
            "S_IFCHR",
            "S_IFBLK",
            "S_IFDIR",
            "S_IFREG",
            "S_IFMT",
            "S_IEXEC",
            "S_IWRITE",
            "S_IREAD",
            "S_IRWXU",
            "S_IXUSR",
            "S_IWUSR",
            "S_IRUSR",
            "F_OK",
            "R_OK",
            "W_OK",
            "X_OK",
            "STDIN_FILENO",
            "STDOUT_FILENO",
            "STDERR_FILENO"), CConstant;

        // Match special comments for region markers
        re!("//BEGIN"), RegionMarker => region_marker;
        re!("//END"), RegionMarker => region_marker;

        // Match comments
        re!("//"), Comment => comment;
        re!(r"/\*"), Comment => comment2;
        re!(r"0x[0-9a-fA-F_]+", rust_int_suf!()), Hex;
        re!(r"0o[0-7_]+", rust_int_suf!()), Octal;
        re!(r"0b[0-1_]+", rust_int_suf!()), Binary;
        re!(r"[0-9][0-9_]*\.[0-9_]*([eE][+-]?[0-9_]+)?(f32|f64|f)?"), Float;
        re!(r"[0-9][0-9_]*", rust_int_suf!()), Decimal;
        re!(r"#\["), Attribute => attribute;
        re!(r"#!\["), Attribute => attribute;
        re!(rust_ident!(), "::"), Scope;
        re!(rust_ident!(), "!"), Macro;
        re!("'", rust_ident!()/*, "(?!')"*/), Lifetime;
        re!(r"[{}\[\]]"), Symbol;
        re!("r\""), String => raw_string;
        re!("r##\""), String => raw_hashed2;
        re!("r#\""), String => raw_hashed1;
        re!("\""), String => string;
        re!(r"'"), Character => character;
        re!(rust_ident!());
    }}
}

impl<'a, I> Rust<'a, I> where I: Iterator<Item=&'a str> {
    normal_like!{normal, NormalText:}
    normal_like!{attribute, Attribute:
        re!(r"\]"), Attribute => return;
    }
    context!{function, Definition:
        re!(r"\s+");
        re!(r"\("), NormalText => return;
        re!(r">"), NormalText => return;
    }
    context!{type_, Definition:
        re!(r"\s+");
        re!(r"[=<;]"), NormalText => return;
    }
    // Rustc allows strings to extend over multiple lines, and the
    // only thing a backshash at end-of-line does is remove the whitespace.
    context!{string, String:
        re!(r"\\"), CharEscape => char_escape;
        re!("\""), String => return;
    }
    context!{raw_string, String:
        re!("\""), String => return;
    }
    // These rules are't complete: they won't match r###"abc"###
    context!{raw_hashed1, String:
        re!("\"#"), String => return;
    }
    context!{raw_hashed2, String:
        re!("\"##"), String => return;
    }
    context!{character, Character => return:
        re!(r"\\"), CharEscape => char_escape;
        re!(r"'"), Character => return;
    }
    context!{char_escape, CharEscape => return:
        re!("[nrt'\"]"), CharEscape => return;
        re!(r"x[0-9a-fA-F]{2}"), CharEscape => return;
        re!(r"u\{[0-9a-fA-F]{1,6}\}"), CharEscape => return;
        re!(r"u[0-9a-fA-F]{4}"), CharEscape => return;
        re!(r"U[0-9a-fA-F]{8}"), CharEscape => return;
        re!(r"\."), Error => return;
    }
    context!{region_marker, RegionMarker => return:}
    context!{comment, Comment => return:
        re!(r"\s+");
    }
    context!{comment2, Comment:
        re!(r"\s+");
        re!(r"\*/"), Comment => return;
    }
}

#[allow(non_upper_case_globals)]
pub mod base_styles {
    use super::Style;

    const DEFAULT: Style = Style { color: [1.0, 1.0, 1.0, 1.0], bold: false };

    macro_rules! styles {
        () => {};
        ($name:ident => ($r:expr,$g:expr,$b:expr) $($rest:tt)*) => {
            styles!($name => color=[$r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0, 1.0] $($rest)*);
        };
        ($name:ident => $($extra:ident=$val:expr),*; $($rest:tt)*) => {
            pub const $name: Style = Style { $($extra: $val,)* ..DEFAULT };
            styles!($($rest)*);
        }
    }

    styles!{
        // BreezeDark.
        Normal => (239,240,241);
        Keyword => (239,240,241), bold=true;
        Function => (246,116,0);
        DataType => (41,128,185);
        DecVal => (246,116,0);
        BaseN => (246,116,0);
        Float => (246,116,0);
        Char => (61,174,233);
        String => (218,68,83);
        Comment => (189,195,199);
        RegionMarker => (41,128,185);
        Others => (39,174,96);
        Error => (218,68,83);

        // Default kate5 colors.
        /*Normal => (31,28,27);
        Keyword => (31,28,27), bold=true;
        Function => (100,74,155);
        DataType => (0,87,174);
        DecVal => (176,128,0);
        BaseN => (176,128,0);
        Float => (176,128,0);
        Char => (146,76,157);
        String => (191,3,3);
        Comment => (137,136,135);
        RegionMarker => (0,87,174);
        Others => (0,110,40);
        Error => (218,68,83);*/
    }
}

#[allow(non_upper_case_globals)]
pub mod styles {
    use super::{Style, base_styles};

    macro_rules! styles {
        () => {};
        ($name:ident => $base:ident; $($rest:tt)*) => {
            pub const $name: Style = base_styles::$base;
            styles!($($rest)*);
        };
        ($name:ident => $base:ident, $($extra:ident=$val:expr),*; $($rest:tt)*) => {
            pub const $name: Style = Style { $($extra: $val,)* ..base_styles::$base };
            styles!($($rest)*);
        }
    }

    styles!{
        NormalText => Normal;
        Keyword => Keyword;
        SelfKw => Function, bold=true;
        Type => DataType, bold=true;
        Trait => DataType, bold=true;
        CType => DataType;
        Constant => Keyword;
        CConstant => Normal;
        Definition => Function;
        Comment => Comment;
        Scope => Others, bold=true;
        Decimal => DecVal;
        Hex => BaseN;
        Octal => BaseN;
        Binary => BaseN;
        Float => Float;
        String => String;
        CharEscape => Char;
        Character => Char;
        Macro => Others;
        Attribute => Others;
        Lifetime => Char, bold=true;
        RegionMarker => RegionMarker;
        Error => Error;

        Symbol => Normal;
    }
}
