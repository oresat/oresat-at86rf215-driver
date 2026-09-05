fn main() -> std::io::Result<()> {
    use oresat_at86rf215_driver::daemon::{run, Profile};
    run(Profile::Uhf)
}