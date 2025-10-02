use rand::Rng;
use ratatui::style::{Color, Modifier, Style};

// Pokemon Type Colors based on official Pokemon game colors
#[derive(Debug, Clone, Copy)]
pub enum PokemonType {
    Electric, // Pikachu - Yellow/Gold
    Fire,     // Charizard - Red/Orange
    Water,    // Blastoise - Blue/Cyan
    Grass,    // Venusaur - Green/Nature
    Psychic,  // Mewtwo - Pink/Purple
    Dragon,   // Dragonite - Orange/Purple
    Ghost,    // Gengar - Purple/Dark
    Normal,   // Eevee - Brown/Cream
    Ice,      // Articuno - Light Blue/White
    Dark,     // Umbreon - Dark/Black
}

impl PokemonType {
    pub fn primary_color(&self) -> Color {
        match self {
            PokemonType::Electric => Color::Rgb(255, 215, 0), // Gold
            PokemonType::Fire => Color::Rgb(255, 69, 0),      // Red-Orange
            PokemonType::Water => Color::Rgb(30, 144, 255),   // Dodger Blue
            PokemonType::Grass => Color::Rgb(34, 139, 34),    // Forest Green
            PokemonType::Psychic => Color::Rgb(255, 20, 147), // Deep Pink
            PokemonType::Dragon => Color::Rgb(138, 43, 226),  // Blue Violet
            PokemonType::Ghost => Color::Rgb(75, 0, 130),     // Indigo
            PokemonType::Normal => Color::Rgb(139, 69, 19),   // Saddle Brown
            PokemonType::Ice => Color::Rgb(175, 238, 238),    // Pale Turquoise
            PokemonType::Dark => Color::Rgb(47, 79, 79),      // Dark Slate Gray
        }
    }

    pub fn secondary_color(&self) -> Color {
        match self {
            PokemonType::Electric => Color::Rgb(255, 255, 0), // Bright Yellow
            PokemonType::Fire => Color::Rgb(255, 140, 0),     // Dark Orange
            PokemonType::Water => Color::Rgb(0, 191, 255),    // Deep Sky Blue
            PokemonType::Grass => Color::Rgb(144, 238, 144),  // Light Green
            PokemonType::Psychic => Color::Rgb(221, 160, 221), // Plum
            PokemonType::Dragon => Color::Rgb(72, 61, 139),   // Dark Slate Blue
            PokemonType::Ghost => Color::Rgb(138, 43, 226),   // Blue Violet
            PokemonType::Normal => Color::Rgb(222, 184, 135), // Burlywood
            PokemonType::Ice => Color::Rgb(240, 248, 255),    // Alice Blue
            PokemonType::Dark => Color::Rgb(25, 25, 112),     // Midnight Blue
        }
    }

    pub fn accent_color(&self) -> Color {
        match self {
            PokemonType::Electric => Color::Rgb(255, 165, 0), // Orange
            PokemonType::Fire => Color::Rgb(220, 20, 60),     // Crimson
            PokemonType::Water => Color::Rgb(64, 224, 208),   // Turquoise
            PokemonType::Grass => Color::Rgb(255, 215, 0),    // Gold
            PokemonType::Psychic => Color::Rgb(218, 112, 214), // Orchid
            PokemonType::Dragon => Color::Rgb(255, 215, 0),   // Gold
            PokemonType::Ghost => Color::Rgb(199, 21, 133),   // Medium Violet Red
            PokemonType::Normal => Color::Rgb(255, 218, 185), // Peach Puff
            PokemonType::Ice => Color::Rgb(176, 196, 222),    // Light Steel Blue
            PokemonType::Dark => Color::Rgb(199, 21, 133),    // Medium Violet Red
        }
    }
}

pub struct PokemonTheme {
    pub current_type: PokemonType,
    pub sparkle_chars: Vec<&'static str>,
    pub animation_frame: usize,
}

impl Default for PokemonTheme {
    fn default() -> Self {
        Self {
            current_type: PokemonType::Electric,
            sparkle_chars: vec!["✨", "⭐", "🌟", "💫", "⚡", "🔥", "💧", "🌿", "🔮", "❄️"],
            animation_frame: 0,
        }
    }
}

impl PokemonTheme {
    pub fn new(pokemon_type: PokemonType) -> Self {
        Self {
            current_type: pokemon_type,
            ..Default::default()
        }
    }

    pub fn get_gradient_colors(&self, steps: usize) -> Vec<Color> {
        let primary = self.current_type.primary_color();
        let secondary = self.current_type.secondary_color();

        // Convert ratatui colors to RGB values for gradient
        let (r1, g1, b1) = match primary {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (255, 215, 0), // Default to gold
        };

        let (r2, g2, b2) = match secondary {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (255, 255, 0), // Default to yellow
        };

        let mut colors = Vec::new();
        for i in 0..steps {
            let ratio = i as f32 / (steps - 1) as f32;
            let r = ((1.0 - ratio) * r1 as f32 + ratio * r2 as f32) as u8;
            let g = ((1.0 - ratio) * g1 as f32 + ratio * g2 as f32) as u8;
            let b = ((1.0 - ratio) * b1 as f32 + ratio * b2 as f32) as u8;
            colors.push(Color::Rgb(r, g, b));
        }
        colors
    }

    pub fn get_sparkle(&mut self) -> &str {
        self.animation_frame = (self.animation_frame + 1) % self.sparkle_chars.len();
        self.sparkle_chars[self.animation_frame]
    }

    pub fn get_random_sparkle() -> &'static str {
        let sparkles = [
            "✨", "⭐", "🌟", "💫", "⚡", "🔥", "💧", "🌿", "🔮", "❄️", "💎", "🌈",
        ];
        let mut rng = rand::thread_rng();
        sparkles[rng.gen_range(0..sparkles.len())]
    }

    pub fn title_style(&self) -> Style {
        Style::default()
            .fg(self.current_type.primary_color())
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED)
    }

    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(self.current_type.secondary_color())
            .add_modifier(Modifier::BOLD)
    }

    pub fn accent_style(&self) -> Style {
        Style::default()
            .fg(self.current_type.accent_color())
            .add_modifier(Modifier::ITALIC)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.current_type.primary_color())
    }

    pub fn success_style(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(0, 255, 127)) // Spring Green
            .add_modifier(Modifier::BOLD)
    }

    pub fn error_style(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(255, 69, 0)) // Red Orange
            .add_modifier(Modifier::BOLD)
    }

    pub fn warning_style(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(255, 215, 0)) // Gold
            .add_modifier(Modifier::BOLD)
    }

    pub fn info_style(&self) -> Style {
        Style::default()
            .fg(Color::Rgb(135, 206, 250)) // Light Sky Blue
            .add_modifier(Modifier::ITALIC)
    }

    // Get Pokemon-themed border characters
    pub fn get_border_set(&self) -> ratatui::symbols::border::Set {
        match self.current_type {
            PokemonType::Electric => ratatui::symbols::border::Set {
                top_left: "⚡",
                top_right: "⚡",
                bottom_left: "⚡",
                bottom_right: "⚡",
                vertical_left: "│",
                vertical_right: "│",
                horizontal_top: "━",
                horizontal_bottom: "━",
            },
            PokemonType::Fire => ratatui::symbols::border::Set {
                top_left: "🔥",
                top_right: "🔥",
                bottom_left: "🔥",
                bottom_right: "🔥",
                vertical_left: "│",
                vertical_right: "│",
                horizontal_top: "━",
                horizontal_bottom: "━",
            },
            PokemonType::Water => ratatui::symbols::border::Set {
                top_left: "💧",
                top_right: "💧",
                bottom_left: "💧",
                bottom_right: "💧",
                vertical_left: "│",
                vertical_right: "│",
                horizontal_top: "━",
                horizontal_bottom: "━",
            },
            PokemonType::Grass => ratatui::symbols::border::Set {
                top_left: "🌿",
                top_right: "🌿",
                bottom_left: "🌿",
                bottom_right: "🌿",
                vertical_left: "│",
                vertical_right: "│",
                horizontal_top: "━",
                horizontal_bottom: "━",
            },
            _ => ratatui::symbols::border::ROUNDED,
        }
    }

    // Cycle through different Pokemon types for variety
    pub fn cycle_type(&mut self) {
        use PokemonType::*;
        self.current_type = match self.current_type {
            Electric => Fire,
            Fire => Water,
            Water => Grass,
            Grass => Psychic,
            Psychic => Dragon,
            Dragon => Ghost,
            Ghost => Normal,
            Normal => Ice,
            Ice => Dark,
            Dark => Electric,
        };
    }
}

// Pokemon ASCII Art Collection
pub struct PokemonArt;

impl PokemonArt {
    pub fn get_pikachu() -> Vec<&'static str> {
        vec![
            "      ░░░░░░░░░░░░░░░░░░░░░░░░░░░",
            "    ░░████░░░░██░░░░░░██░░░░████░░",
            "  ░░██████░░██████░░██████░░██████░░",
            "  ░░██████████████████████████████░░",
            "░░████████████████████████████████░░",
            "░░██████████████████████████████████",
            "░░██████████░░████████░░██████████░░",
            "░░██████████░░████████░░██████████░░",
            "░░████████████████████████████████░░",
            "░░██████████████████████████████░░░░",
            "  ░░██████████████████████████░░░░",
            "  ░░████████░░██████░░██████░░░░",
            "    ░░██████░░░░██░░░░░░░░░░░░",
            "      ░░░░░░░░░░░░░░░░░░░░░░",
        ]
    }

    pub fn get_charizard() -> Vec<&'static str> {
        vec![
            "                    ░░░░████░░░░",
            "                ░░████████████░░",
            "              ░░██████████████░░",
            "            ░░████████████████░░░░",
            "          ░░██████████████████░░",
            "        ░░████████████████████░░░░",
            "      ░░██████████░░██░░██████░░",
            "    ░░████████████░░██░░████████░░",
            "    ░░██████████████████████████░░",
            "  ░░████████████████████████████░░",
            "  ░░██████████████████████████░░░░",
            "    ░░████████████████████░░░░░░",
            "      ░░██████████████░░░░░░",
            "        ░░░░██████░░░░░░",
        ]
    }

    pub fn get_blastoise() -> Vec<&'static str> {
        vec![
            "          ░░░░░░░░░░░░░░░░",
            "        ░░████████████████░░",
            "      ░░██████████████████░░",
            "    ░░████████████████████░░░░",
            "    ░░████████░░░░░░████████░░",
            "  ░░██████░░░░░░░░░░░░██████░░",
            "  ░░████░░░░██░░░░██░░░░████░░",
            "░░████░░░░████░░░░████░░░░████░░",
            "░░██░░░░██████░░░░██████░░░░██░░",
            "░░░░░░████████████████████░░░░░░",
            "  ░░██████████████████████░░",
            "    ░░████████████████░░░░",
            "      ░░██████████░░░░",
            "        ░░░░░░░░░░",
        ]
    }

    pub fn get_venusaur() -> Vec<&'static str> {
        vec![
            "        ░░░░░░██░░░░░░░░░░",
            "      ░░████████████████░░",
            "    ░░██████░░░░░░██████░░",
            "  ░░██████░░░░░░░░░░██████░░",
            "  ░░████░░░░██░░██░░░░████░░",
            "░░████░░░░████░░████░░░░████░░",
            "░░██░░░░██████░░██████░░░░██░░",
            "░░░░░░████████████████████░░░░",
            "  ░░██████████████████████░░",
            "    ░░████████████████░░░░",
            "      ░░██████████░░░░",
            "        ░░██████░░",
            "          ░░░░░░",
        ]
    }

    pub fn get_pokeball() -> Vec<&'static str> {
        vec![
            "    ░░░░░░░░░░░░░░░░",
            "  ░░██████████████░░",
            "░░████████████████░░",
            "░░██████░░░░██████░░",
            "░░████░░░░░░░░████░░",
            "░░██░░░░████░░░░██░░",
            "░░░░░░████████░░░░░░",
            "░░██░░░░████░░░░██░░",
            "░░████░░░░░░░░████░░",
            "░░██████░░░░██████░░",
            "░░████████████████░░",
            "  ░░██████████████░░",
            "    ░░░░░░░░░░░░░░░░",
        ]
    }

    pub fn get_eevee() -> Vec<&'static str> {
        vec![
            "      ░░░░██░░░░░░░░██░░░░",
            "    ░░██████░░░░░░██████░░",
            "  ░░████████░░░░░░████████░░",
            "  ░░██████████████████████░░",
            "░░████████████████████████░░",
            "░░██████████░░░░██████████░░",
            "░░██████████░░░░██████████░░",
            "░░████████████████████████░░",
            "░░██████████████████████░░░░",
            "  ░░██████████████████░░░░",
            "    ░░████████████░░░░",
            "      ░░██████░░░░",
            "        ░░░░░░",
        ]
    }

    pub fn get_random_pokemon() -> Vec<&'static str> {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..6) {
            0 => Self::get_pikachu(),
            1 => Self::get_charizard(),
            2 => Self::get_blastoise(),
            3 => Self::get_venusaur(),
            4 => Self::get_eevee(),
            _ => Self::get_pokeball(),
        }
    }
}

// Enhanced Loading Animation
pub struct PokemonLoader {
    pub frames: Vec<String>,
    pub current_frame: usize,
    pub pokemon_type: PokemonType,
}

impl PokemonLoader {
    pub fn new(pokemon_type: PokemonType) -> Self {
        let base_frames = match pokemon_type {
            PokemonType::Electric => vec![
                "⚡    ".to_string(),
                " ⚡   ".to_string(),
                "  ⚡  ".to_string(),
                "   ⚡ ".to_string(),
                "    ⚡".to_string(),
                "   ⚡ ".to_string(),
                "  ⚡  ".to_string(),
                " ⚡   ".to_string(),
            ],
            PokemonType::Fire => vec![
                "🔥    ".to_string(),
                " 🔥   ".to_string(),
                "  🔥  ".to_string(),
                "   🔥 ".to_string(),
                "    🔥".to_string(),
                "   🔥 ".to_string(),
                "  🔥  ".to_string(),
                " 🔥   ".to_string(),
            ],
            PokemonType::Water => vec![
                "💧    ".to_string(),
                " 💧   ".to_string(),
                "  💧  ".to_string(),
                "   💧 ".to_string(),
                "    💧".to_string(),
                "   💧 ".to_string(),
                "  💧  ".to_string(),
                " 💧   ".to_string(),
            ],
            _ => vec![
                "✨    ".to_string(),
                " ✨   ".to_string(),
                "  ✨  ".to_string(),
                "   ✨ ".to_string(),
                "    ✨".to_string(),
                "   ✨ ".to_string(),
                "  ✨  ".to_string(),
                " ✨   ".to_string(),
            ],
        };

        Self {
            frames: base_frames,
            current_frame: 0,
            pokemon_type,
        }
    }

    pub fn next_frame(&mut self) -> &str {
        let frame = &self.frames[self.current_frame];
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        frame
    }
}
