use std::{error::Error, path::PathBuf};
use mini_proost::process_input;
 fn main()  {

    let input = "def add : Nat -> Nat := λ x, x + 1";
    let res = process_input(input, PathBuf::new()).expect("Bad");

}