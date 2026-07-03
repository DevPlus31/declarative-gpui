//! Shared data model — both implementations render exactly this.

pub struct Item {
    pub name: String,
    pub role: String,
    pub accent: u32,
    pub active: bool,
}

pub fn sample_items(n: usize) -> Vec<Item> {
    const NAMES: &[&str] = &[
        "Aya Kern",
        "Bruno Malta",
        "Chidi Okafor",
        "Dana Vieira",
        "Emre Sahin",
        "Farah Idris",
        "Goran Ilic",
        "Hana Sato",
        "Ines Duarte",
        "Jonas Weber",
        "Kofi Mensah",
        "Lina Haddad",
    ];
    const ROLES: &[&str] = &[
        "Rust Engineer",
        "Product Designer",
        "GPU Wrangler",
        "Tech Writer",
        "SRE",
        "Compiler Nerd",
    ];
    const ACCENTS: &[u32] = &[0xe0b184, 0x9ece6a, 0x7aa2f7, 0xf7768e, 0xbb9af7, 0x73daca];

    (0..n)
        .map(|i| Item {
            name: format!("{} {:02}", NAMES[i % NAMES.len()], i + 1),
            role: ROLES[i % ROLES.len()].to_string(),
            accent: ACCENTS[i % ACCENTS.len()],
            active: i % 3 != 0,
        })
        .collect()
}
