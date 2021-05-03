use std::error::Error;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::{fs, thread, time};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "ex-post-progress")]
struct Opt {
    pid: u64,
    paths: Vec<PathBuf>,
}

fn find_fds_for_open_file(
    pid: u64,
    paths: &[PathBuf],
) -> Result<Vec<(u32, PathBuf)>, Box<dyn Error>> {
    let mut fds = vec![];
    for dir_entry in fs::read_dir(format!("/proc/{}/fd/", pid))? {
        let dir_entry = dir_entry?;
        let proc_fd_path = dir_entry.path();
        let dest_path = proc_fd_path.read_link()?;
        if paths.contains(&dest_path)
            || (paths.is_empty() && proc_fd_path.metadata()?.file_type().is_file())
        {
            fds.push((
                dir_entry
                    .file_name()
                    .into_string()
                    .unwrap()
                    .parse::<u32>()
                    .unwrap(),
                dest_path,
            ));
        }
    }
    Ok(fds)
}

fn get_pos_from_fdinfo(contents: &str) -> u64 {
    for line in contents.lines() {
        if line.starts_with("pos:") {
            let mut pieces = line.split('\t');
            pieces.next();
            return pieces.next().unwrap().parse::<u64>().unwrap();
        }
    }
    panic!("Couldn't parse fdinfo")
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let absolute_paths = opt
        .paths
        .iter()
        .map(|p| fs::canonicalize(p).unwrap())
        .collect::<Vec<_>>();
    let pid = opt.pid;
    let fds = find_fds_for_open_file(pid, &absolute_paths)?;

    let m = indicatif::MultiProgress::new();

    for (fd, path) in fds {
        let file_size = fs::metadata(format!("/proc/{}/fd/{}", pid, fd))?.len();
        let pb = m.add(indicatif::ProgressBar::new(file_size));
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
                .progress_chars("#>-"),
        );
        pb.set_message(format!(
            "/proc/{}/fd/{} => {}",
            pid,
            fd,
            path.file_name().unwrap().to_string_lossy()
        ));
        let mut fdinfo = fs::File::open(format!("/proc/{}/fdinfo/{}", pid, fd))?;
        #[allow(clippy::verbose_file_reads)]
        thread::spawn(move || {
            let mut contents = String::new();
            loop {
                contents.clear();
                if let Err(e) = fdinfo.read_to_string(&mut contents) {
                    if e.kind() == io::ErrorKind::NotFound {
                        break;
                    } else {
                        panic!("{:?}", e)
                    }
                }
                fdinfo.seek(SeekFrom::Start(0)).unwrap();

                let pos = get_pos_from_fdinfo(&contents);
                pb.set_position(pos);
                if pos == file_size {
                    break;
                }
                thread::sleep(time::Duration::from_millis(100));
            }
            pb.finish_with_message("done");
        });
    }
    m.join_and_clear().unwrap();

    Ok(())
}
