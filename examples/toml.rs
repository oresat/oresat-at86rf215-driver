use oresat_at86rf215_driver::radio::*;
use oresat_at86rf215_driver::registers::*;
use toml;


fn main() {

    // Radio initialization (from simple_radio example)
    let mut radio = Radio::new();
    radio.rf_cfg.value = radio.rf_cfg.value.with_drv(3).with_irqmm(true);
    radio.rf_cfg.value.set_irqp(true);
    let mut pending_writes = BulkWrites::new();
    pending_writes.add(&mut radio.rf_cfg);
    pending_writes.add(&mut radio.rf09_cmd);
    pending_writes.add(&mut radio.rf_clko);
    let write_commands = pending_writes.generate_commands();
    for (idx, command) in write_commands.into_iter().enumerate() {
        println!("Bulk Write command [{}]: {:02X?}", idx, command)
    }

    // Creates config object from radio
    let config = radio.to_config();

    let toml_string = toml::to_string(&config).expect("Failed to serialize config to TOML");

    std::fs::write("radio.toml", toml_string).expect("Failed to write radio.toml");

    println!("Radio written to radio.toml");
}
