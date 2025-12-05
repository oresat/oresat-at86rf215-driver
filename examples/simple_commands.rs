use oresat_at86rf215_driver::registers::*;

fn main() {
    // Create an RfCfg bitfield with some settings
    let cfg = RfCfg::new().with_drv(3).with_irqp(true).with_irqmm(true);

    // Wrap it in the ReadWrite register wrapper
    let rf_cfg = ReadWrite::<RfCfg, 0x0006, 1>::new(cfg);

    let cmd = rf_cfg.write_command();
    println!("Write command: {:02X?}", cmd);

    let cmd = rf_cfg.read_command();
    println!("Read command: {:02X?}", cmd);
}
