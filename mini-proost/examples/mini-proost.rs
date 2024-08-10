use std::error::Error;
use mini_proost::process_input;
 fn main()  {

    let input = "1 + 1";
    let res = process_input(input).expect("Bad");

}