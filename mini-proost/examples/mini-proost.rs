use mini_proost::process_input;
 fn main()  {

    let input = "def x: Prop := True";
    let res = process_input(input).expect("Bad");

}