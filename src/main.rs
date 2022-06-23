use std::{self, ffi::CString, path};

// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let command = &args[3];
    let command_args = &args[4..];

    let base = "/tmp/mydocker";
    let base_path = path::Path::new(base);

    // prevent cryptic "no such file or directory" error inside chroot
    std::fs::create_dir_all(base_path.join("dev")).unwrap();
    std::fs::File::create(base_path.join("dev/null")).unwrap();

    // copy over binary into chroot
    let command_path = path::Path::new(command).strip_prefix("/").unwrap();
    std::fs::create_dir_all(base_path.join(command_path.parent().unwrap()))
        .expect("Failed to create directory for executed binary");
    std::fs::copy(command, base_path.join(command_path))
        .expect("Failed copying executed binary to chroot directory");

    // create and change into chroot
    let cbase_path = CString::new(base.to_owned()).unwrap();
    unsafe {
        libc::chroot(cbase_path.as_ptr());
    }

    // ensure that directory changed to root of jail
    std::env::set_current_dir("/").expect("Failed to change to root dir");

    unsafe {
        libc::unshare(libc::CLONE_NEWPID);
    }

    let output = std::process::Command::new(command)
        .args(command_args)
        .output()
        .unwrap();

    let std_out = std::str::from_utf8(&output.stdout).unwrap();
    print!("{}", std_out);
    let std_err = std::str::from_utf8(&output.stderr).unwrap();
    eprint!("{}", std_err);

    match output.status.code() {
        Some(code) => std::process::exit(code),
        None => std::process::exit(1),
    }
}
