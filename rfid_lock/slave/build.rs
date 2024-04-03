use std::fs;

use esp_idf_part::{AppType, DataType, Partition, PartitionTable, SubType, Type};

// const LITTLEFS_SUBTYPE: u8 = 0x83;

fn main() -> eyre::Result<()> {
    embuild::espidf::sysenv::output();

    let partitions = vec![
        Partition::new(
            "nvs",
            Type::Data,
            SubType::Data(DataType::Nvs),
            0x9_000, // the first 0x8000 bytes (32KB) of the partition are reserved for the bootloader,
            // then another 0x1000 bytes (4KB) for the partition table,
            // so the first partition must start at 0x9000
            0x6_000, // Data partitions must be multiples of 0x1000 (4KB)
            false,
        ),
        Partition::new(
            "phy_init",
            Type::Data,
            SubType::Data(DataType::Phy),
            0xf_000,
            0x1_000,
            false,
        ),
        Partition::new(
            "factory",
            Type::App,
            SubType::App(AppType::Factory),
            0x10_000,  // App offsets must be multiples of 0x10000 (64KB)
            0x200_000, // 2MB factory partition
            false,
        ),
        Partition::new(
            "fs",
            Type::Data,
            SubType::Data(DataType::Fat),
            0x210_000, // App offsets must be multiples of 0x10000 (64KB)
            0x100_000, // 1MB filesystem partition
            false,
        ),
    ];

    let table = PartitionTable::new(partitions);

    let csv = table.to_csv()?;
    let bin = table.to_bin()?;

    fs::write("./partition/4mb.csv", csv)?;
    fs::write("./partition/4mb.bin", bin)?;

    Ok(())
}
