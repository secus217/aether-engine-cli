use crate::pokemon_theme::{PokemonArt, PokemonTheme, PokemonType};
use rand::Rng;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
        Wrap,
    },
};

// Pokeball Progress Bar Widget
pub struct PokeballProgress {
    pub percent: f64,
    pub label: Option<String>,
    pub pokemon_type: PokemonType,
    pub animated: bool,
    pub sparkles: bool,
}

impl PokeballProgress {
    pub fn new(percent: f64) -> Self {
        Self {
            percent: percent.clamp(0.0, 100.0),
            label: None,
            pokemon_type: PokemonType::Electric,
            animated: false,
            sparkles: false,
        }
    }

    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn pokemon_type(mut self, ptype: PokemonType) -> Self {
        self.pokemon_type = ptype;
        self
    }

    pub fn animated(mut self) -> Self {
        self.animated = true;
        self
    }

    pub fn sparkles(mut self) -> Self {
        self.sparkles = true;
        self
    }
}

impl Widget for PokeballProgress {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = PokemonTheme::new(self.pokemon_type);

        // Create pokeball-style progress bar
        let progress_width = ((area.width as f64 * self.percent / 100.0) as u16).min(area.width);

        // Background - safely check bounds
        for x in 0..area.width {
            for y in 0..area.height {
                let pos_x = area.x + x;
                let pos_y = area.y + y;

                // Check bounds before accessing buffer
                if pos_x < buf.area().width && pos_y < buf.area().height {
                    if let Some(cell) = buf.cell_mut((pos_x, pos_y)) {
                        if x < progress_width {
                            cell.set_fg(theme.current_type.primary_color())
                                .set_bg(theme.current_type.secondary_color())
                                .set_symbol("●");
                        } else {
                            cell.set_fg(Color::Gray).set_symbol("○");
                        }
                    }
                }
            }
        }

        // Add sparkles if enabled
        if self.sparkles && self.percent > 0.0 {
            let mut rng = rand::thread_rng();
            for _ in 0..((progress_width / 4).max(1)) {
                let x = rng.gen_range(0..progress_width);
                let y = rng.gen_range(0..area.height);
                let pos_x = area.x + x;
                let pos_y = area.y + y;

                // Check bounds before accessing buffer
                if pos_x < buf.area().width && pos_y < buf.area().height {
                    if let Some(cell) = buf.cell_mut((pos_x, pos_y)) {
                        let sparkle = PokemonTheme::get_random_sparkle();
                        cell.set_symbol(sparkle).set_fg(Color::Rgb(255, 255, 255));
                    }
                }
            }
        }

        // Add label if provided
        if let Some(label) = self.label {
            let text = format!("{} {}%", label, self.percent as u8);
            if area.height > 0 {
                let label_area = Rect {
                    x: area.x + 1,
                    y: area.y + area.height / 2,
                    width: area.width.saturating_sub(2),
                    height: 1.min(area.height),
                };

                // Only render if label area is valid
                if label_area.x < buf.area().width && label_area.y < buf.area().height {
                    Paragraph::new(text)
                        .style(theme.accent_style().add_modifier(Modifier::BOLD))
                        .alignment(Alignment::Center)
                        .render(label_area, buf);
                }
            }
        }
    }
}

// Pokemon Type Badge Widget
pub struct TypeBadge {
    pub pokemon_type: PokemonType,
    pub text: String,
    pub animated: bool,
}

impl TypeBadge {
    pub fn new<S: Into<String>>(pokemon_type: PokemonType, text: S) -> Self {
        Self {
            pokemon_type,
            text: text.into(),
            animated: false,
        }
    }

    pub fn animated(mut self) -> Self {
        self.animated = true;
        self
    }
}

impl Widget for TypeBadge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = PokemonTheme::new(self.pokemon_type);

        let type_symbol = match self.pokemon_type {
            PokemonType::Electric => "⚡",
            PokemonType::Fire => "🔥",
            PokemonType::Water => "💧",
            PokemonType::Grass => "🌿",
            PokemonType::Psychic => "🔮",
            PokemonType::Dragon => "🐉",
            PokemonType::Ghost => "👻",
            PokemonType::Normal => "⭐",
            PokemonType::Ice => "❄️",
            PokemonType::Dark => "🌙",
        };

        let badge_text = if self.animated {
            format!(
                "{} {} {}",
                type_symbol,
                self.text,
                PokemonTheme::get_random_sparkle()
            )
        } else {
            format!("{} {}", type_symbol, self.text)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style())
            .style(Style::default().bg(theme.current_type.primary_color().into()));

        let paragraph = Paragraph::new(badge_text)
            .style(theme.title_style())
            .alignment(Alignment::Center)
            .block(block);

        paragraph.render(area, buf);
    }
}

// Pokemon Status Widget
pub struct PokemonStatus {
    pub hp: f64,
    pub mp: f64,
    pub level: u32,
    pub name: String,
    pub pokemon_type: PokemonType,
    pub status_effects: Vec<String>,
}

impl PokemonStatus {
    pub fn new<S: Into<String>>(name: S, pokemon_type: PokemonType) -> Self {
        Self {
            hp: 100.0,
            mp: 100.0,
            level: 1,
            name: name.into(),
            pokemon_type,
            status_effects: Vec::new(),
        }
    }

    pub fn hp(mut self, hp: f64) -> Self {
        self.hp = hp.clamp(0.0, 100.0);
        self
    }

    pub fn mp(mut self, mp: f64) -> Self {
        self.mp = mp.clamp(0.0, 100.0);
        self
    }

    pub fn level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    pub fn add_status<S: Into<String>>(mut self, status: S) -> Self {
        self.status_effects.push(status.into());
        self
    }
}

impl Widget for PokemonStatus {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = PokemonTheme::new(self.pokemon_type);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(3), // Name and level
                Constraint::Length(3), // HP bar
                Constraint::Length(3), // MP bar
                Constraint::Min(1),    // Status effects
            ])
            .split(area);

        // Name and Level
        let type_symbol = match self.pokemon_type {
            PokemonType::Electric => "⚡",
            PokemonType::Fire => "🔥",
            PokemonType::Water => "💧",
            PokemonType::Grass => "🌿",
            PokemonType::Psychic => "🔮",
            PokemonType::Dragon => "🐉",
            PokemonType::Ghost => "👻",
            PokemonType::Normal => "⭐",
            PokemonType::Ice => "❄️",
            PokemonType::Dark => "🌙",
        };

        let header_text = format!("{} {} • Lv.{}", type_symbol, self.name, self.level);
        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style())
            .title("Pokemon Status");

        Paragraph::new(header_text)
            .style(theme.title_style())
            .alignment(Alignment::Center)
            .block(header_block)
            .render(chunks[0], buf);

        // HP Bar
        let hp_color = if self.hp > 50.0 {
            Color::Rgb(0, 255, 0) // Green
        } else if self.hp > 20.0 {
            Color::Rgb(255, 255, 0) // Yellow
        } else {
            Color::Rgb(255, 0, 0) // Red
        };

        let hp_gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border_style())
                    .title("❤️ HP"),
            )
            .gauge_style(Style::default().fg(hp_color))
            .percent(self.hp as u16)
            .label(format!("{:.0}/100", self.hp));

        hp_gauge.render(chunks[1], buf);

        // MP Bar
        let mp_gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border_style())
                    .title("💙 MP"),
            )
            .gauge_style(Style::default().fg(Color::Rgb(0, 100, 255)))
            .percent(self.mp as u16)
            .label(format!("{:.0}/100", self.mp));

        mp_gauge.render(chunks[2], buf);

        // Status Effects
        if !self.status_effects.is_empty() {
            let status_text: Vec<Line> = self
                .status_effects
                .iter()
                .map(|effect| {
                    Line::from(vec![
                        Span::styled("✨ ", theme.accent_style()),
                        Span::styled(effect, theme.info_style()),
                    ])
                })
                .collect();

            let status_block = Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border_style())
                .title("Status Effects");

            Paragraph::new(status_text)
                .block(status_block)
                .wrap(Wrap { trim: true })
                .render(chunks[3], buf);
        }
    }
}

// Pokemon Battle Animation Widget
#[derive(Debug, Clone)]
pub struct BattleAnimation {
    pub attacker: String,
    pub defender: String,
    pub move_name: String,
    pub animation_frame: usize,
    pub pokemon_type: PokemonType,
    pub is_critical: bool,
}

impl BattleAnimation {
    pub fn new<S: Into<String>>(
        attacker: S,
        defender: S,
        move_name: S,
        pokemon_type: PokemonType,
    ) -> Self {
        Self {
            attacker: attacker.into(),
            defender: defender.into(),
            move_name: move_name.into(),
            animation_frame: 0,
            pokemon_type,
            is_critical: false,
        }
    }

    pub fn critical(mut self) -> Self {
        self.is_critical = true;
        self
    }

    pub fn next_frame(&mut self) {
        self.animation_frame = (self.animation_frame + 1) % 8;
    }
}

impl Widget for BattleAnimation {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = PokemonTheme::new(self.pokemon_type);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(area);

        // Attack animation text
        let attack_effects = match self.pokemon_type {
            PokemonType::Electric => vec!["⚡", "🌟", "✨", "💫"],
            PokemonType::Fire => vec!["🔥", "💥", "🌋", "☄️"],
            PokemonType::Water => vec!["💧", "🌊", "💦", "🌀"],
            PokemonType::Grass => vec!["🌿", "🌳", "🍃", "🌺"],
            _ => vec!["✨", "💫", "⭐", "🌟"],
        };

        let effect = attack_effects[self.animation_frame % attack_effects.len()];

        let battle_text = if self.is_critical {
            format!(
                "💥 CRITICAL HIT! 💥\n{} used {}!\n{} {} {} {} {}",
                self.attacker, self.move_name, effect, effect, effect, effect, effect
            )
        } else {
            format!(
                "{} used {}!\n{} {} {}",
                self.attacker, self.move_name, effect, effect, effect
            )
        };

        let battle_style = if self.is_critical {
            theme.error_style().add_modifier(Modifier::RAPID_BLINK)
        } else {
            theme.title_style()
        };

        let battle_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style())
            .title("⚔️ Battle!");

        Paragraph::new(battle_text)
            .style(battle_style)
            .alignment(Alignment::Center)
            .block(battle_block)
            .render(chunks[1], buf);

        // Show some ASCII art if space allows
        if chunks[0].height >= 8 && chunks[0].width >= 30 {
            let art = PokemonArt::get_random_pokemon();
            let art_text: Vec<Line> = art
                .into_iter()
                .take(chunks[0].height as usize)
                .map(|line| Line::from(Span::styled(line, theme.accent_style())))
                .collect();

            Paragraph::new(art_text)
                .alignment(Alignment::Center)
                .render(chunks[0], buf);
        }
    }
}

// Enhanced Pokemon-themed List Widget
pub struct PokemonList<'a> {
    pub items: Vec<String>,
    pub pokemon_type: PokemonType,
    pub title: Option<String>,
    pub selected_style: Style,
    pub highlight_symbol: &'a str,
    pub animated: bool,
}

impl<'a> PokemonList<'a> {
    pub fn new(items: Vec<String>, pokemon_type: PokemonType) -> Self {
        let theme = PokemonTheme::new(pokemon_type);
        Self {
            items,
            pokemon_type,
            title: None,
            selected_style: theme.title_style().add_modifier(Modifier::REVERSED),
            highlight_symbol: "🔥 ",
            animated: false,
        }
    }

    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn animated(mut self) -> Self {
        self.animated = true;
        self
    }

    pub fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = symbol;
        self
    }
}

impl<'a> StatefulWidget for PokemonList<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let theme = PokemonTheme::new(self.pokemon_type);

        let type_symbol = match self.pokemon_type {
            PokemonType::Electric => "⚡",
            PokemonType::Fire => "🔥",
            PokemonType::Water => "💧",
            PokemonType::Grass => "🌿",
            PokemonType::Psychic => "🔮",
            PokemonType::Dragon => "🐉",
            PokemonType::Ghost => "👻",
            PokemonType::Normal => "⭐",
            PokemonType::Ice => "❄️",
            PokemonType::Dark => "🌙",
        };

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(_i, item)| {
                let content = if self.animated {
                    format!(
                        "{} {} {}",
                        type_symbol,
                        item,
                        PokemonTheme::get_random_sparkle()
                    )
                } else {
                    format!("{} {}", type_symbol, item)
                };

                ListItem::new(content).style(theme.info_style())
            })
            .collect();

        let title = self.title.unwrap_or_else(|| "Pokemon List".to_string());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border_style())
            .title(title);

        let list = List::new(items)
            .block(block)
            .style(theme.info_style())
            .highlight_style(self.selected_style)
            .highlight_symbol(self.highlight_symbol);

        StatefulWidget::render(list, area, buf, state);
    }
}

// Pokemon-themed notification popup
#[derive(Debug, Clone)]
pub struct PokemonNotification {
    pub message: String,
    pub notification_type: NotificationType,
    pub pokemon_type: PokemonType,
    pub auto_dismiss: bool,
}

#[derive(Debug, Clone)]
pub enum NotificationType {
    Success,
    Error,
    Warning,
    Info,
    Critical,
}

impl PokemonNotification {
    pub fn success<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Success,
            pokemon_type: PokemonType::Grass,
            auto_dismiss: true,
        }
    }

    pub fn error<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Error,
            pokemon_type: PokemonType::Fire,
            auto_dismiss: false,
        }
    }

    pub fn warning<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Warning,
            pokemon_type: PokemonType::Electric,
            auto_dismiss: true,
        }
    }

    pub fn info<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Info,
            pokemon_type: PokemonType::Water,
            auto_dismiss: true,
        }
    }

    pub fn critical<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Critical,
            pokemon_type: PokemonType::Dark,
            auto_dismiss: false,
        }
    }
}

impl Widget for PokemonNotification {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = PokemonTheme::new(self.pokemon_type);

        let (icon, title, style) = match self.notification_type {
            NotificationType::Success => ("✅", "Success", theme.success_style()),
            NotificationType::Error => ("❌", "Error", theme.error_style()),
            NotificationType::Warning => ("⚠️", "Warning", theme.warning_style()),
            NotificationType::Info => ("ℹ️", "Info", theme.info_style()),
            NotificationType::Critical => (
                "💀",
                "Critical",
                theme.error_style().add_modifier(Modifier::RAPID_BLINK),
            ),
        };

        let notification_text = format!("{} {}\n\n{}", icon, title, self.message);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(format!(
                "{} Pokemon Notification {}",
                PokemonTheme::get_random_sparkle(),
                PokemonTheme::get_random_sparkle()
            ));

        let paragraph = Paragraph::new(notification_text)
            .style(style)
            .alignment(Alignment::Center)
            .block(block)
            .wrap(Wrap { trim: true });

        // Clear the area first
        Clear.render(area, buf);
        paragraph.render(area, buf);
    }
}
