static VERSION: &str = env!("CARGO_PKG_VERSION");

use std::{env, io};
use std::fs::{create_dir_all, read_dir, copy, remove_file};
use io::Write;
use std::path::Path;
use std::time::Duration;
use std::process::{Command, Stdio};

mod tz_info;
mod util;
use util::{create_cfg, create_start_script, dl_file, extract_tar, install_deps, verify_file};

fn main() -> io::Result<()> {

    println!("Free Palestine 🍉️ 🇵🇸️ \n\n");
    println!("\x1b[1;32mO\x1b[0m\x1b[1;95mrbix\x1b[0m v{} \n", &VERSION);

    println!("\nIt is recommended to update your system before running Orbix.");
    println!("If you didn't do that, you can quit Orbix right now by pressing Ctrl + C.");
    println!("If your system is upto-date, press ENTER\n");
    io::stdin().read_line(&mut String::new())?;

    let args: Vec<String> = env::args().collect();

    let target = if args.len() == 2
    {
        args[1].clone()
    }
    else if args.len() == 1
    {
        println!("\x1b[1;94mEnter the target directory. It should be an absolute path and empty directory!\x1b[0m");
        print!("Target Directory: ");
        io::stdout().flush()?;
        let mut target = String::new();
        io::stdin().read_line(&mut target)?;
        let target = target.trim().to_string();

        target
    }
    else
    {
        eprintln!("Usage: {} target", args[0]);
        std::process::exit(1);
    };

    let target_path = Path::new(&target);

    if !target_path.exists() || !target_path.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Target is not a directory"));
    }
    {
        match read_dir(target_path) {
            Ok(entries) => {
                // Check if there are any entries in the directory
                for _ in entries {
                    return Err(io::Error::new(io::ErrorKind::Other, "Target directory is not empty"));
                }
            }
            Err(_) => {
                eprintln!("Failed to read the directory: {}", target_path.to_str().unwrap());
                return Err(io::Error::new(io::ErrorKind::Other, "Failed to read the directory"));
            }
        }
    }
    let e = create_dirs(target_path);
    e.expect("I couldn't create necessary directories");

    println!("\n\x1b[1;94mOkay, I will check and download server files if needed.");
    println!("Downloading is done only once if files remain intact.\x1b[0m");

    println!("");

    print!("Starting in \x1b[1;93m3\x1b[0m..."); io::stdout().flush()?;
    std::thread::sleep(Duration::from_secs(1));
    print!("\rStarting in \x1b[1;93m2\x1b[0m..."); io::stdout().flush()?;
    std::thread::sleep(Duration::from_secs(1));
    print!("\rStarting in \x1b[1;93m1\x1b[0m..."); io::stdout().flush()?;
    std::thread::sleep(Duration::from_secs(1));
    println!("\rStarting...     \n");
    dl_files().unwrap();

    println!("\n\x1b[1;94mDone downloading files.\x1b[0m\n");
    std::thread::sleep(Duration::from_millis(350));

    create_links(target_path).expect("Failed to create symlinks for pak files");

    println!("\n\x1b[1;94mCreated symlinks for paks.\x1b[0m\n");
    std::thread::sleep(Duration::from_millis(350));

    println!("\n\x1b[1;94mCopying rest of the files.\x1b[0m\n");
    rest_of_files(target_path).unwrap();
    std::thread::sleep(Duration::from_millis(350));

    println!("\n\x1b[1;94mInstalling dependencies...\x1b[0m\n");
    install_deps()?;
    println!("\n\x1b[1;94mDone installing dependencies.\x1b[0m\n");
    std::thread::sleep(Duration::from_millis(350));

    println!("\n\x1b[1;94mCompiling iw1x-server...\x1b[0m\n");
    compile_iw1x(target_path)?;
    std::thread::sleep(Duration::from_millis(350));

    println!("\n\x1b[1;94mCreating start script...\x1b[0m\n");
    create_start_script(target_path)?;
    std::thread::sleep(Duration::from_millis(350));

    println!("\n\x1b[1;94mAll done.\x1b[0m\n");

    Ok(())
}

fn create_dirs(targetdir: &Path) -> io::Result<()>
{
    let datadir = Path::new(&std::env::var("HOME").unwrap())
        .join(".local/share/orbix/main");
    create_dir_all(datadir)?;
    create_dir_all(targetdir.join("main"))?;

    Ok(())
}

fn create_links(dst: &Path) -> io::Result<()>
{
    let srcdir = std::path::Path::new(&std::env::var("HOME").unwrap())
        .join(".local/share/orbix/main");

    let maindir = dst.join("main");

    for entry in read_dir(srcdir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().unwrap();
            let target_link = maindir.join(file_name);
            println!("Linking \x1b[1;96m{}\x1b[0m → \x1b[1;95m{}\x1b[0m", path.to_str().unwrap(), target_link.to_str().unwrap());
            if !target_link.exists() {
                std::os::unix::fs::symlink(&path, &target_link)?;
            }
        }
    }

    Ok(())
}

fn rest_of_files(targetdir: &Path) -> io::Result<()>
{
    let svpath = std::path::Path::new(&std::env::var("HOME").unwrap())
        .join(".local/share/orbix/cod-lnxded-1.1d.tar.bz2");

    create_cfg(&targetdir.join("main/myserver.cfg"))?;
    copy(svpath, targetdir.join("cod-lnxded-1.1d.tar.bz2"))?;
    extract_tar(&targetdir.join("cod-lnxded-1.1d.tar.bz2"), targetdir)?;
    remove_file(targetdir.join("cod-lnxded-1.1d.tar.bz2"))?;
    Ok(())
}

fn dl_files() -> io::Result<()>
{
    let basefiles = vec![
        ("localized_english_pak0.pk3", "f103bd8e20785ecaed1fed5be0c0fe16"),
        ("localized_english_pak1.pk3", "d2d481075ab68cb579693501a0b5e900"),
        ("pak0.pk3", "03b8bd99d7a5ba7d02a456cbe83fc1a9"),
        ("pak1.pk3", "88d699f4b6a4dd0af5dc42711205c192"),
        ("pak2.pk3", "4079eee442ad0b649e97a5f9258cbc2a"),
        ("pak3.pk3", "61db72b5728fb70b0f2b4d0f1ef4c804"),
        ("pak4.pk3", "c077d4be6cb56e88417f58b516690fb9"),
        ("pak5.pk3", "0cb20baa66ddecc72ccb7f17b3062bb3"),
        ("pak6.pk3", "d8b721be71c0f21fc98901c12cccf3ec"),
    ];

    let path_ = std::path::Path::new(&env::var("HOME").unwrap())
        .join(".local/share/orbix");
    for (file, sum) in &basefiles {
        let outpath = path_.join("main").join(&file);

        while !verify_file(sum, &outpath).unwrap() {
            dl_file(format!("https://de.dvotx.org/dump/cod1/basefiles/{}", &file).as_str(), &outpath).expect("Failed to download basefiles");
        }
    }

    let svpath = path_.join("cod-lnxded-1.1d.tar.bz2");
    let svsum = "5269ed44b0da8692cd5e61c76b6823d7";
    while !verify_file(svsum, &svpath).unwrap() {
        dl_file("https://de.dvotx.org/dump/cod1/cod-lnxded-1.1d.tar.bz2", &svpath).expect("Failed to download cod-lnxded-1.1d.tar.bz2");
    }
    Ok(())
}

fn compile_iw1x(targetdir: &Path) -> io::Result<()>
{
    let path_ = std::path::Path::new(&env::var("HOME").unwrap())
        .join(".local/share/orbix");

    let script = path_.join("compile_iw1x");
    let cmd: &str = script.to_str().unwrap();
    Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    let binpath = path_.join("iw1x-server/bin");
    copy(binpath.join("iw1x.so"), targetdir.join("iw1x.so")).expect(&format!("Failed to copy {}/iw1x.so to {}", binpath.to_str().unwrap(), targetdir.to_str().unwrap()));

    Ok(())
}

