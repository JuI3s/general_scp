use std::path::{Path, PathBuf};

const REL_TEST_DATA_DIR: &str = "test_data";

pub fn project_src_path() -> PathBuf {
    let output = String::from_utf8(
        std::process::Command::new(env!("CARGO"))
            .arg("locate-project")
            .arg("--workspace")
            .arg("--message-format=plain")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    let (output, _) = output.rsplit_once("/").unwrap();
    let path = Path::new(output).join("src");

    path
}

pub fn test_data_dir() -> PathBuf {
    project_src_path().join(REL_TEST_DATA_DIR)
}
