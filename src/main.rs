use std::{self, ffi::CString, path};

mod registry;

// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let image_name = &args[2];
    let command = &args[3];
    let command_args = &args[4..];

    let base_path = std::env::temp_dir().join("docker");

    // prevent cryptic "no such file or directory" error inside chroot
    std::fs::create_dir_all(base_path.join("dev")).unwrap();
    std::fs::File::create(base_path.join("dev/null")).unwrap();

    let image = registry::ImageIdentifier::from_string(image_name);
    let mut reg = registry::Registry::default();
    reg.pull(&image, base_path.to_str().unwrap());

    // copy over binary into chroot directory
    let command_path = path::Path::new(command).strip_prefix("/").unwrap();
    std::fs::create_dir_all(base_path.join(command_path.parent().unwrap()))
        .expect("Failed to create directory for executed binary");
    std::fs::copy(command, base_path.join(command_path))
        .expect("Failed copying executed binary to chroot directory");

    // create and change into chroot directory
    let cbase_path = CString::new(base_path.to_str().unwrap().to_owned()).unwrap();
    unsafe {
        libc::chroot(cbase_path.as_ptr());
    }

    // ensure that directory changed to root of jail
    std::env::set_current_dir("/").expect("Failed to change to root dir");

    // `unshare` puts the next created process in a seperate PID namespace
    unsafe {
        libc::unshare(libc::CLONE_NEWPID);
    }

    let output = std::process::Command::new(command)
        .args(command_args)
        .output()
        .unwrap();

    print!("{}", std::str::from_utf8(&output.stdout).unwrap());
    eprint!("{}", std::str::from_utf8(&output.stderr).unwrap());

    std::process::exit(output.status.code().unwrap_or(1));
}
