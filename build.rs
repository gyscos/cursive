extern crate skeptic;

fn main() {
    extern crate skeptic;

    skeptic::generate_doc_tests(&[
        "Readme.md",
        "doc/tutorial_1.md",
        "doc/tutorial_2.md",
        "doc/tutorial_3.md",
    ]);
}
