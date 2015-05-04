extern crate syntax;
extern crate rustc;
extern crate rustc_driver;
extern crate rustc_resolve as resolve;
extern crate rustc_typeck as typeck;

use self::syntax::{ast_map, codemap, diagnostic};
pub use self::syntax::diagnostic::Level;
use self::syntax::ast_map::NodePrinter;
use self::syntax::print::pprust;
use self::rustc::session::{self, config};
use self::rustc::metadata::creader::CrateReader;
use self::rustc::middle::{self, stability, ty};
use self::rustc::util::ppaux::UserString;
use self::rustc_driver::driver;

use std::env;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

#[cfg(windows)]
const EXE_SUFFIX: &'static str = "exe";
#[cfg(not(windows))]
const EXE_SUFFIX: &'static str = "";

fn get_rustc_dir_path() -> PathBuf {
    if cfg!(windows) {
        env::current_exe().unwrap().parent().unwrap().to_path_buf()
    } else {
        let out = Command::new("which").arg("rustc").output().unwrap();
        if !out.status.success() {
            panic!("Failed to get rustc path: {}", String::from_utf8(out.stderr).unwrap());
        }
        Path::new(str::from_utf8(&out.stdout).unwrap().trim()).parent().unwrap().to_path_buf()
    }
}

pub fn init_env() {
    if cfg!(windows) {
        // Set up %PATH% to be able to run the bundled `rustc.exe`.
        if let Some(path) = env::var_os("PATH") {
            let mut paths = env::split_paths(&path).collect::<Vec<_>>();
            paths.push(get_rustc_dir_path());
            let new_path = env::join_paths(paths.iter()).unwrap();
            env::set_var("PATH", &new_path);
        }
    }
}

pub fn compile_and_run(path: &Path) {
    let path = env::current_dir().unwrap().join(path);
    assert!(path.is_absolute());
    let exe = path.with_extension(EXE_SUFFIX);

    // There seems to be a bug that Rust programs (including `rustc.exe`) ran with inherited
    // std{in,out,err} via `.status()` or `.spawn()` produce a "`xyz.exe` has stopped working"
    // message unconditionally.
    if cfg!(windows) {
        let out = Command::new("rustc").arg(&path).arg("-o").arg(&exe).output().unwrap();
        println!("{}\n{}", String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap());
        if !out.status.success() {
            println!("compilation failed");
        } else {
            match Command::new(&exe).output() {
                Err(e) => {
                    println!("failed to execute `{:?}`: {}", exe, e);
                }
                Ok(out) => {
                    println!("{}\n{}", String::from_utf8(out.stdout).unwrap(),
                                       String::from_utf8(out.stderr).unwrap());
                }
            }
        }
    } else {
        if let Ok(status) = Command::new("rustc").arg(&path).arg("-o").arg(&exe).status() {
            if status.success() {
                if let Err(e) = Command::new(&exe).spawn() {
                    println!("failed to execute `{:?}`: {}", exe, e);
                }
            }
        }
    }
}

enum Req {
    TypesAtOffset(usize, Range<usize>)
}

enum Res {
    Done,
    Aborted,
    Diagnostic(Diagnostic),
    TypesAtOffset(usize, Vec<(Range<usize>, String)>)
}

struct Diagnostic {
    line: usize,
    col: usize,
    level: Level,
    message: String
}

struct ErrorLogger {
    tx: Sender<Res>,
    file_end: usize
}

impl diagnostic::Emitter for ErrorLogger {
    fn emit(&mut self,
            cmsp: Option<(&codemap::CodeMap, codemap::Span)>,
            msg: &str, _code: Option<&str>, lvl: Level) {
        if msg.starts_with("aborting due to ") {
            return;
        }
        let (line, col) = match cmsp {
            Some((codemap, sp)) => {
                if sp.hi.0 as usize <= self.file_end {
                    let pos = codemap.lookup_char_pos(sp.lo);
                    (pos.line - 1, pos.col.0)
                } else {
                    (0, 0)
                }
            }
            None => (0, 0)
        };
        let _ = self.tx.send(Res::Diagnostic(Diagnostic {
            line: line,
            col: col,
            level: lvl,
            message: msg.to_owned()
        }));
    }

    fn custom_emit(&mut self, cm: &codemap::CodeMap,
                   sp: diagnostic::RenderSpan, msg: &str, lvl: Level) {
        let sp = match sp {
            diagnostic::FullSpan(s) | diagnostic::FileLine(s) |
            diagnostic::EndSpan(s) | diagnostic::Suggestion(s, _)  => s
        };
        self.emit(Some((cm, sp)), msg, None, lvl);
    }
}

fn rustc_thread(input: String, mut lifeline: Arc<()>, rx: Receiver<Req>, tx: Sender<Res>, emitter: ErrorLogger) {
    macro_rules! still_alive {
        () => (lifeline = match lifeline.downgrade().upgrade() { Some(x) => x, None => return })
    }
    let input = config::Input::Str(input);

    let rustc_dir_path = get_rustc_dir_path();

    let sessopts = config::Options {
        maybe_sysroot: Some(rustc_dir_path.parent().unwrap().to_path_buf()),
        ..config::basic_options().clone()
    };

    let codemap = codemap::CodeMap::new();
    let diagnostic_handler = diagnostic::mk_handler(true, Box::new(emitter));
    let span_diagnostic_handler = diagnostic::mk_span_handler(diagnostic_handler, codemap);

    let sess = session::build_session_(sessopts,
                                       None,
                                       span_diagnostic_handler);

    let cfg = config::build_configuration(&sess);

    still_alive!();
    let krate = driver::phase_1_parse_input(&sess, cfg, &input);

    still_alive!();
    let krate = driver::phase_2_configure_and_expand(&sess, krate, "r3", None)
        .expect("phase_2_configure_and_expand aborted");

    let mut forest = ast_map::Forest::new(krate);
    let arenas = ty::CtxtArenas::new();

    still_alive!();
    let ast_map = driver::assign_node_ids_and_map(&sess, &mut forest);
    let krate = ast_map.krate();

    still_alive!();
    CrateReader::new(&sess).read_crates(krate);
    let lang_items = middle::lang_items::collect_language_items(krate, &sess);

    still_alive!();
    let resolve::CrateMap {
        def_map,
        freevars,
        trait_map,
        ..
    } = resolve::resolve_crate(&sess,
                               &ast_map,
                               &lang_items,
                               krate,
                               resolve::MakeGlobMap::No);

    // Discard MTWT tables that aren't required past resolution.
    syntax::ext::mtwt::clear_tables();

    still_alive!();
    let named_region_map = middle::resolve_lifetime::krate(&sess, krate, &def_map);

    still_alive!();
    let region_map = middle::region::resolve_crate(&sess, krate);

    //middle::check_loop::check_crate(&sess, krate);

    //middle::check_static_recursion::check_crate(&sess, krate, &def_map, &ast_map);

    still_alive!();
    let tcx = &ty::mk_ctxt(sess,
                           &arenas,
                           def_map,
                           named_region_map,
                           ast_map,
                           freevars,
                           region_map,
                           lang_items,
                           stability::Index::new(krate));
    typeck::check_crate(tcx, trait_map);

    let _ = tx.send(Res::Done);

    for req in rx.iter() {
        still_alive!();
        match req {
            Req::TypesAtOffset(offset, line) => {
                let mut out = vec![];
                for (&id, ty) in tcx.node_types().iter() {
                    let node =  if let Some(node) = tcx.map.find(id) {
                        node
                    } else {
                        continue;
                    };
                    // Avoid statements, they're always ().
                    if let ast_map::NodeStmt(_) = node {
                        continue;
                    }
                    if let Some(sp) = tcx.map.opt_span(id) {
                        /*
                        // Avoid peeking at macro expansions.
                        if sp.expn_id != codemap::NO_EXPANSION {
                            continue;
                        }*/

                        let (lo, hi) = (sp.lo.0 as usize, sp.hi.0 as usize);
                        if line.start <= lo && lo <= offset && offset <= hi && hi <= line.end {
                            match node {
                                // These cannot be reliably printed.
                                ast_map::NodeLocal(_) | ast_map::NodeArg(_) | ast_map::NodeStructCtor(_) => {}
                                // There is an associated NodeExpr(ExprBlock) where this actually matters.
                                ast_map::NodeBlock(_) => continue,
                                _ => {
                                    let node_string = pprust::to_string(|s| s.print_node(&node));
                                    let span_string = tcx.sess.codemap().span_to_snippet(sp).unwrap();
                                    let is_macro = regex!(r"^\w+\s*!\s*[\(\[\{]").is_match(&span_string);
                                    if !is_macro && node_string.replace(" ", "") != span_string.replace(" ", "") {
                                        continue;
                                    }
                                }
                            }
                            let clean = regex!(concat![r"\b(",
                                "core::(option::Option|result::Result)|",
                                "collections::(vec::Vec|string::String)",
                            r")\b"]);
                            let ty_string = clean.replace_all(&ty.user_string(tcx), |c: &::regex::Captures| {
                                c.at(0).unwrap().split(':').next_back().unwrap().to_owned()
                            });
                            out.push((lo-line.start..hi-line.start, ty_string));
                        }
                    }
                }
                let _ = tx.send(Res::TypesAtOffset(offset, out));
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum State {
    Compiling,
    Aborted,
    Waiting,
    TypesAtOffset(usize)
}

pub struct Rustc {
    pub file_end: usize,
    _lifeline: Arc<()>,
    req_tx: Sender<Req>,
    res_rx: Receiver<Res>,
    pub state: State,
    // True if error.
    pub diagnostics: HashMap<usize, Vec<(Level, usize, String)>>,
    pub errors: usize,
    pub types_at_offset: Option<Vec<(Range<usize>, String)>>
}

impl Rustc {
    pub fn start(input: String) -> Rustc {
        let lifeline = Arc::new(());
        let lifeline2 = lifeline.clone();
        let (req_tx, req_rx) = channel();
        let (res_tx, res_rx) = channel();
        let input_len = input.len();
        thread::spawn(move || {
            let res_tx2 = res_tx.clone();
            let res = thread::catch_panic(move || {
                rustc_thread(input, lifeline2, req_rx, res_tx.clone(), ErrorLogger {
                    tx: res_tx,
                    file_end: input_len
                });
            });
            if res.is_err() {
                let _ = res_tx2.send(Res::Aborted);
            }
        });
        Rustc {
            file_end: input_len,
            _lifeline: lifeline,
            req_tx: req_tx,
            res_rx: res_rx,
            state: State::Compiling,
            diagnostics: HashMap::new(),
            errors: 0,
            types_at_offset: None
        }
    }

    pub fn update(&mut self) -> bool {
        let mut dirty = false;
        while let Ok(res) = self.res_rx.try_recv() {
            match res {
                Res::Done => {
                    assert_eq!(self.state, State::Compiling);
                    self.state = State::Waiting;
                }
                Res::Aborted => {
                    assert!(self.errors > 0, "aborted without errors?!");
                    self.state = State::Aborted;
                }
                Res::Diagnostic(d) => {
                     match d.level {
                        Level::Bug | Level::Fatal | Level::Error => {
                            self.errors += 1;
                        }
                        _ => {}
                    }
                    self.diagnostics.entry(d.line).or_insert(vec![]).push((d.level, d.col, d.message));
                    dirty = true;
                }
                Res::TypesAtOffset(offset, result) => {
                    if self.state == State::TypesAtOffset(offset) {
                        self.state = State::Waiting;
                        self.types_at_offset = Some(result);
                        dirty = true;
                    }
                }
            }
        }
        dirty
    }

    pub fn types_at_offset(&mut self, offset: usize, line: Range<usize>) {
        self.types_at_offset = None;
        self.state = State::TypesAtOffset(offset);
        let _ = self.req_tx.send(Req::TypesAtOffset(offset, line));
    }
}
