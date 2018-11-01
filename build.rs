fn main() {
    #[cfg(feature = "ncurses-backend")]
    {
        extern crate ncurses;
        if ncurses::NCURSES_MOUSE_VERSION == 1 {
            print!(r#"cargo:rustc-cfg=feature="ncurses.mouse_v1""#);
        }
    }
}
