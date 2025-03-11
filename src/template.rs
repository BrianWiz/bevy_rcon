use maud::{html, Markup, DOCTYPE};

pub struct TemplateParams {
    pub tab_title: String,
    pub game_name: String,
    pub server_name: String,
    pub content: Markup,
}

pub fn base_template(params: TemplateParams) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title { (params.tab_title) }
                script src="https://unpkg.com/htmx.org@1.9.10" {}
                link href="https://fonts.googleapis.com/css2?family=Poppins:wght@400;600&display=swap" rel="stylesheet" {}
            }
            body {
                h1 { (params.game_name) }
                h2 { (params.server_name) }
                (params.content)
            }
        }
    }
}
