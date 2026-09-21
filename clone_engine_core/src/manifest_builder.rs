fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn float(value: f64) -> String {
    let text = format!("{value:?}");
    if text.contains('.') || text.contains('e') {
        text
    } else {
        format!("{text}.0")
    }
}

pub struct Motion {
    lines: Vec<String>,
}

impl Motion {
    pub fn new(name: &str, animation: &str) -> Self {
        Self {
            lines: vec![
                format!("name = {}", quoted(name)),
                format!("animation = {}", quoted(animation)),
            ],
        }
    }

    fn text(mut self, key: &str, value: &str) -> Self {
        self.lines.push(format!("{key} = {}", quoted(value)));
        self
    }

    fn number(mut self, key: &str, value: impl core::fmt::Display) -> Self {
        self.lines.push(format!("{key} = {value}"));
        self
    }

    pub fn template(self, template: &str) -> Self {
        self.text("template", template)
    }

    pub fn scripts(self, stem: &str) -> Self {
        self.text("scripts", stem)
    }

    pub fn game(self, script: &str) -> Self {
        self.text("game", script)
    }

    pub fn sound(self, script: &str) -> Self {
        self.text("sound", script)
    }

    pub fn effect(self, script: &str) -> Self {
        self.text("effect", script)
    }

    pub fn expression(self, script: &str) -> Self {
        self.text("expression", script)
    }

    pub fn flags(mut self, flags: &[&str]) -> Self {
        let list: Vec<String> = flags.iter().map(|flag| quoted(flag)).collect();
        self.lines.push(format!("flags = [{}]", list.join(", ")));
        self
    }

    pub fn blend_frames(self, frames: u8) -> Self {
        self.number("blend_frames", frames)
    }

    pub fn cancel_frame(self, frame: u8) -> Self {
        self.number("cancel_frame", frame)
    }

    pub fn xlu(mut self, start: u8, end: u8) -> Self {
        self.lines.push(format!("xlu = [{start}, {end}]"));
        self
    }

    pub fn no_stop_intp(self, value: bool) -> Self {
        self.number("no_stop_intp", value)
    }

    pub fn animation_unk(self, value: u8) -> Self {
        self.number("animation_unk", value)
    }

    pub fn no_extra(self) -> Self {
        self.number("no_extra", true)
    }
}

pub struct Manifest {
    name: String,
    fighter: Vec<String>,
    articles: Vec<Vec<String>>,
    kirby: Vec<String>,
    motions: Vec<Motion>,
    meshes: Vec<(String, bool)>,
    params: Vec<(Option<u32>, bool, String, String)>,
    raw: Vec<String>,
}

impl Manifest {
    pub fn new(name: &str, base: &str) -> Self {
        Self {
            name: name.to_string(),
            fighter: vec![
                format!("name = {}", quoted(name)),
                format!("base = {}", quoted(base)),
            ],
            articles: Vec::new(),
            kirby: Vec::new(),
            motions: Vec::new(),
            meshes: Vec::new(),
            params: Vec::new(),
            raw: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn text(mut self, key: &str, value: &str) -> Self {
        self.fighter.push(format!("{key} = {}", quoted(value)));
        self
    }

    fn number(mut self, key: &str, value: impl core::fmt::Display) -> Self {
        self.fighter.push(format!("{key} = {value}"));
        self
    }

    pub fn costumes(self, count: u32) -> Self {
        self.number("costumes", count)
    }

    pub fn color_start(self, first: u32) -> Self {
        self.number("color_start", first)
    }

    pub fn series(self, series: &str) -> Self {
        self.text("series", series)
    }

    pub fn disp_order(self, order: i8) -> Self {
        self.number("disp_order", order)
    }

    pub fn save_no(self, save_no: i8) -> Self {
        self.number("save_no", save_no)
    }

    pub fn exhibit_year(self, year: i16) -> Self {
        self.number("exhibit_year", year)
    }

    pub fn narration(self, label: &str) -> Self {
        self.text("narration", label)
    }

    pub fn name_label(self, label: &str) -> Self {
        self.text("display_name", label)
    }

    pub fn staffroll(self) -> Self {
        self.number("staffroll", true)
    }

    pub fn own_css(self) -> Self {
        self.number("css", false)
    }

    pub fn jingle(self, name: &str) -> Self {
        self.text("jingle", name)
    }

    pub fn ui_chara(self, ui_chara: &str) -> Self {
        self.text("ui_chara", ui_chara)
    }

    pub fn fighter_kind_name(self, name: &str) -> Self {
        self.text("fighter_kind_name", name)
    }

    pub fn resource_name(self, name: &str) -> Self {
        self.text("resource_name", name)
    }

    pub fn base_resource_name(self, name: &str) -> Self {
        self.text("base_resource_name", name)
    }

    pub fn owns_param_resources(self) -> Self {
        self.number("owns_param_resources", true)
    }

    pub fn effect_namespace(self, namespace: u32) -> Self {
        self.number("effect_namespace", namespace)
    }

    pub fn article_namespace(self, namespace: u32) -> Self {
        self.number("article_namespace", namespace)
    }

    pub fn article(mut self, name: &str, from: &str) -> Self {
        self.articles.push(vec![
            format!("name = {}", quoted(name)),
            format!("from = {}", quoted(from)),
        ]);
        self
    }

    pub fn base_article(mut self, name: &str, base_article: &str) -> Self {
        self.articles.push(vec![
            format!("name = {}", quoted(name)),
            format!("base_article = {}", quoted(base_article)),
        ]);
        self
    }

    pub fn kirby_article(mut self, name: &str, from: &str) -> Self {
        self.articles.push(vec![
            format!("name = {}", quoted(name)),
            format!("from = {}", quoted(from)),
            "kirby = true".to_string(),
        ]);
        self
    }

    pub fn kirby(mut self, statuses: i32) -> Self {
        self.kirby.push(format!("statuses = {statuses}"));
        self
    }

    pub fn kirby_first(mut self, first: i32) -> Self {
        self.kirby.push(format!("first = {first}"));
        self
    }

    pub fn kirby_full_model(mut self) -> Self {
        self.kirby.push("full_model = true".to_string());
        self
    }

    pub fn kirby_model(mut self, model: &str) -> Self {
        self.kirby.push(format!("model = {}", quoted(model)));
        self
    }

    pub fn kirby_mesh(mut self, name: &str, visible: bool) -> Self {
        self.meshes.push((name.to_string(), visible));
        self
    }

    pub fn kirby_motion(mut self, motion: Motion) -> Self {
        self.motions.push(motion);
        self
    }

    fn push_param(mut self, costume: Option<u32>, mul: bool, name: &str, value: String) -> Self {
        self.params.push((costume, mul, name.to_string(), value));
        self
    }

    pub fn param(self, name: &str, value: f64) -> Self {
        self.push_param(None, false, name, float(value))
    }

    pub fn param_int(self, name: &str, value: i32) -> Self {
        self.push_param(None, false, name, value.to_string())
    }

    pub fn param_mul(self, name: &str, value: f64) -> Self {
        self.push_param(None, true, name, float(value))
    }

    pub fn param_at(self, costume: u32, name: &str, value: f64) -> Self {
        self.push_param(Some(costume), false, name, float(value))
    }

    pub fn param_int_at(self, costume: u32, name: &str, value: i32) -> Self {
        self.push_param(Some(costume), false, name, value.to_string())
    }

    pub fn param_mul_at(self, costume: u32, name: &str, value: f64) -> Self {
        self.push_param(Some(costume), true, name, float(value))
    }

    pub fn params(mut self, values: &[(&str, f64)]) -> Self {
        for (name, value) in values {
            self = self.param(name, *value);
        }
        self
    }

    pub fn params_int(mut self, values: &[(&str, i32)]) -> Self {
        for (name, value) in values {
            self = self.param_int(name, *value);
        }
        self
    }

    pub fn params_mul(mut self, values: &[(&str, f64)]) -> Self {
        for (name, value) in values {
            self = self.param_mul(name, *value);
        }
        self
    }

    pub fn raw(mut self, toml: &str) -> Self {
        self.raw.push(toml.to_string());
        self
    }

    pub fn to_toml(&self) -> String {
        let mut out = String::from("[fighter]\n");
        for line in &self.fighter {
            out.push_str(line);
            out.push('\n');
        }
        for article in &self.articles {
            out.push_str("\n[[article]]\n");
            for line in article {
                out.push_str(line);
                out.push('\n');
            }
        }
        if !self.kirby.is_empty() || !self.motions.is_empty() || !self.meshes.is_empty() {
            out.push_str("\n[kirby]\n");
            for line in &self.kirby {
                out.push_str(line);
                out.push('\n');
            }
            for motion in &self.motions {
                out.push_str("\n[[kirby.motion]]\n");
                for line in &motion.lines {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            for (name, visible) in &self.meshes {
                out.push_str(&format!("\n[[kirby.mesh]]\nname = {}\nvisible = {visible}\n", quoted(name)));
            }
        }
        let mut tables: Vec<(Option<u32>, bool)> = Vec::new();
        for (costume, mul, _, _) in &self.params {
            if !tables.contains(&(*costume, *mul)) {
                tables.push((*costume, *mul));
            }
        }
        tables.sort();
        for (costume, mul) in tables {
            let mut header = String::from("params");
            if let Some(costume) = costume {
                header.push_str(&format!(".c{costume:02}"));
            }
            if mul {
                header.push_str(".mul");
            }
            out.push_str(&format!("\n[{header}]\n"));
            for (param_costume, param_mul, name, value) in &self.params {
                if *param_costume == costume && *param_mul == mul {
                    out.push_str(&format!("{name} = {value}\n"));
                }
            }
        }
        for raw in &self.raw {
            out.push('\n');
            out.push_str(raw);
            if !raw.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }
}

pub struct ItemManifest {
    resource_name: String,
    item: Vec<String>,
    common: Vec<String>,
    owner_params: Vec<String>,
    raw: Vec<String>,
}

impl ItemManifest {
    pub fn new(resource_name: &str, base_kind: i32) -> Self {
        Self {
            resource_name: resource_name.to_string(),
            item: vec![
                format!("base_kind = {base_kind}"),
                format!("resource_name = {}", quoted(resource_name)),
            ],
            common: Vec::new(),
            owner_params: Vec::new(),
            raw: Vec::new(),
        }
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    fn text(mut self, key: &str, value: &str) -> Self {
        self.item.push(format!("{key} = {}", quoted(value)));
        self
    }

    fn number(mut self, key: &str, value: impl core::fmt::Display) -> Self {
        self.item.push(format!("{key} = {value}"));
        self
    }

    pub fn base_item(self, name: &str) -> Self {
        self.text("base_item", name)
    }

    pub fn agent_name(self, name: &str) -> Self {
        self.text("agent_name", name)
    }

    pub fn ui_id(self, ui_id: &str) -> Self {
        self.text("ui_id", ui_id)
    }

    pub fn training_order(self, order: i32) -> Self {
        self.number("training_order", order)
    }

    pub fn spawn_per(self, per: i32) -> Self {
        self.number("spawn_per", per)
    }

    pub fn spawn_range(self, min: i32, max: i32) -> Self {
        self.number("spawn_min", min).number("spawn_max", max)
    }

    pub fn spawn_from(mut self, names: &[&str]) -> Self {
        self.item.push(format!("spawn_from = {}", quoted(&names.join(","))));
        self
    }

    pub fn common(mut self, field: &str, value: f64) -> Self {
        self.common.push(format!("{field} = {}", float(value)));
        self
    }

    pub fn common_int(mut self, field: &str, value: i32) -> Self {
        self.common.push(format!("{field} = {value}"));
        self
    }

    pub fn owner_param(mut self, owner: &str, field: &str, value: f64) -> Self {
        self.owner_params.push(format!("{owner}.{field} = {}", float(value)));
        self
    }

    pub fn owner_param_int(mut self, owner: &str, field: &str, value: i32) -> Self {
        self.owner_params.push(format!("{owner}.{field} = {value}"));
        self
    }

    pub fn raw(mut self, toml: &str) -> Self {
        self.raw.push(toml.to_string());
        self
    }

    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        for line in &self.item {
            out.push_str(line);
            out.push('\n');
        }
        if !self.common.is_empty() {
            out.push_str("\n[common]\n");
            for line in &self.common {
                out.push_str(line);
                out.push('\n');
            }
        }
        if !self.owner_params.is_empty() {
            out.push_str("\n[owner_params]\n");
            for line in &self.owner_params {
                out.push_str(line);
                out.push('\n');
            }
        }
        for raw in &self.raw {
            out.push('\n');
            out.push_str(raw);
            if !raw.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_renders_the_tables_the_engine_reads() {
        let text = Manifest::new("bluster", "donkey")
            .costumes(8)
            .narration("vc_narration_characall_bluster")
            .article("cannonballcloned", "koopajr/cannonball")
            .kirby_article("blustercannonball", "koopajr/cannonball")
            .kirby(1)
            .kirby_first(0x410)
            .kirby_motion(Motion::new("bluster_special_n", "blusterd00specialn.nuanmb").template("donkey_special_n"))
            .param("dash_speed", 2.0)
            .param_int("jump_squat_frame", 3)
            .param_mul("common.shield_size", 3.0)
            .param_at(3, "weight", 100.0)
            .to_toml();
        let expected = "[fighter]\nname = \"bluster\"\nbase = \"donkey\"\ncostumes = 8\nnarration = \"vc_narration_characall_bluster\"\n\n[[article]]\nname = \"cannonballcloned\"\nfrom = \"koopajr/cannonball\"\n\n[[article]]\nname = \"blustercannonball\"\nfrom = \"koopajr/cannonball\"\nkirby = true\n\n[kirby]\nstatuses = 1\nfirst = 1040\n\n[[kirby.motion]]\nname = \"bluster_special_n\"\nanimation = \"blusterd00specialn.nuanmb\"\ntemplate = \"donkey_special_n\"\n\n[params]\ndash_speed = 2.0\njump_squat_frame = 3\n\n[params.mul]\ncommon.shield_size = 3.0\n\n[params.c03]\nweight = 100.0\n";
        assert_eq!(text, expected);
    }

    #[test]
    fn the_rendered_text_parses_back_into_the_same_declaration() {
        let text = Manifest::new("bluster", "donkey")
            .costumes(8)
            .series("donkey")
            .disp_order(2)
            .article("cannonballcloned", "koopajr/cannonball")
            .kirby_article("blustercannonball", "koopajr/cannonball")
            .kirby(1)
            .kirby_first(0x410)
            .kirby_motion(Motion::new("bluster_special_n", "blusterd00specialn.nuanmb").template("donkey_special_n"))
            .params(&[("walk_accel_mul", 0.105), ("dash_speed", 2.0)])
            .params_int(&[("jump_squat_frame", 3)])
            .params_mul(&[("common.shield_size", 3.0)])
            .to_toml();
        let parsed = crate::fighter_toml::parse_all(&text).expect("the builder writes what the parser reads");
        assert_eq!(parsed.len(), 1);
        let fighter = &parsed[0];
        assert!(!fighter.own_css);
        assert!(crate::fighter_toml::parse_all(&Manifest::new("a", "mario").own_css().to_toml()).unwrap()[0].own_css);
        assert_eq!(fighter.name, "bluster");
        assert_eq!(fighter.base, "donkey");
        assert_eq!(fighter.color_count, 8);
        assert_eq!(fighter.disp_order, Some(2));
        assert_eq!(fighter.articles.len(), 2);
        assert!(fighter.articles[1].kirby);
        let kirby = fighter.kirby.as_ref().expect("kirby table");
        assert_eq!((kirby.statuses, kirby.first), (1, Some(0x410)));
        assert_eq!(kirby.motions[0].template.as_deref(), Some("donkey_special_n"));
        assert_eq!(fighter.params.len(), 4);
        assert_eq!(fighter.params[2].name, "jump_squat_frame");
        assert!(matches!(fighter.params[2].value, crate::fighter_toml::ParamValue::Int(3)));
        assert!(matches!(fighter.params[3].op, crate::fighter_toml::ParamOp::Mul));
    }

    #[test]
    fn floats_always_carry_a_point_and_strings_are_escaped() {
        assert_eq!(float(2.0), "2.0");
        assert_eq!(float(0.07337), "0.07337");
        assert_eq!(quoted("a\"b"), "\"a\\\"b\"");
        let text = ItemManifest::new("blusterblock", 430)
            .base_item("pickelobject")
            .owner_param_int("pickel", "life", 600)
            .owner_param("pickel", "auto_damage", 0.0)
            .to_toml();
        assert_eq!(
            text,
            "base_kind = 430\nresource_name = \"blusterblock\"\nbase_item = \"pickelobject\"\n\n[owner_params]\npickel.life = 600\npickel.auto_damage = 0.0\n"
        );
    }
}
