fn main() {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "Klein") {
        println!("{}", dirs.config_dir().join("klein.log").display());
    }
}
