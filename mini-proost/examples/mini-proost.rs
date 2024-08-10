use std::{error::Error, path::PathBuf};
use mini_proost::process_input;
 fn main()  {

    let input = "def x: Prop := True";
    let res = process_input(input, PathBuf::new()).expect("Bad");

}