use std::env;
use xenith::native::{self, NativeError};
fn fail(error: NativeError) -> ! {
    eprintln!("error: {error}");
    std::process::exit(1)
}
fn main() {
    let mut args = env::args_os();
    let _ = args.next();
    let Some(command) = args.next() else {
        fail(NativeError::usage(
            "expected `new`, `check`, `build`, or `run`",
        ));
    };
    match command.to_string_lossy().as_ref() {
        "new" => {
            let Some(name) = args.next() else {
                fail(NativeError::usage("`xenith new` requires a package name"));
            };
            if args.next().is_some() {
                fail(NativeError::usage("`xenith new` accepts one package name"));
            }
            let root = env::current_dir().unwrap_or_else(|e| fail(NativeError::io(e)));
            native::project::create_project(&root, &name.to_string_lossy())
                .unwrap_or_else(|error| fail(error));
        }
        "check" | "build" | "run" => {
            let root = env::current_dir().unwrap_or_else(|e| fail(NativeError::io(e)));
            let project =
                native::project::Project::discover(&root).unwrap_or_else(|error| fail(error));
            fail(NativeError::not_implemented(format!(
                "`{command:?}` for {}",
                project.root.display()
            )));
        }
        _ => fail(NativeError::usage(
            "expected `new`, `check`, `build`, or `run`",
        )),
    }
}
