fn main() -> std::io::Result<()> {
    oresat_at86rf215_driver::daemon::run(oresat_at86rf215_driver::Profile::Uhf)
}