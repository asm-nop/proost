use mini_proost::process_input;
 fn main()  {
    let input = "def x: Prop := True";
    process_input(input).expect("Bad");
}