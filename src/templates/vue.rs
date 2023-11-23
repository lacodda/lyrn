use super::{ProjectProps, Template};
use crate::libs::types::Content;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn get(project: &ProjectProps) -> Template {
    Template {
        dependencies: dependencies(),
        dev_dependencies: dev_dependencies(),
        eslintrc: eslintrc(),
        app: app(&project),
        ..Template::default()
    }
}

fn dependencies() -> Value {
    json!({
        "vue": "^3.3.4",
        "vue-router": "^4.2.4"
    })
}

fn dev_dependencies() -> Value {
    json!({
        "@vue/eslint-config-typescript": "^11.0.3",
        "eslint-plugin-vue": "^9.14.1",
        "vue-loader": "^17.1.1",
    })
}

fn eslintrc() -> Value {
    json!({
        "extends": [
            "plugin:vue/recommended",
            "@vue/eslint-config-typescript"
        ],
        "rules": {
            "vue/attribute-hyphenation": "off",
            "vue/component-definition-name-casing": ["warn", "kebab-case"],
            "vue/comment-directive": "off",
            "vue/html-indent": ["warn", 2, { "alignAttributesVertically": false }],
            "vue/html-self-closing": "off",
            "vue/max-attributes-per-line": ["warn", { "singleline": 10, "multiline": 10 }],
            "vue/multi-word-component-names": "off",
            "vue/mustache-interpolation-spacing": ["warn", "never"],
            "vue/singleline-html-element-content-newline": "off",
            "vue/v-slot-style": "off",
        },
        "settings": {
            "import/resolver": {
              "node": {
                "extensions": [".vue"]
              }
            }
          }
    })
}

fn app(_project: &ProjectProps) -> HashMap<&'static str, Content> {
    HashMap::from([("src/main.ts", Content::Str(main())), ("src/routes.ts", Content::Str(routes()))])
}

fn main() -> String {
    r###"import { createApp } from 'vue';
import App from '@/components/App.vue';
import routes from '@/routes';
import ui from '@/ui';

createApp(App).use(ui).use(routes).mount('#app');
"###
    .into()
}

fn routes() -> String {
    r###"import { createRouter, createWebHistory } from 'vue-router';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'Home',
      component: async () => await import('./components/Home.vue'),
    },
    {
      path: '/about',
      name: 'About',
      component: async () => await import('./components/About.vue'),
    },
    {
      path: '/info',
      name: 'Info',
      component: async () => await import('./components/Info.vue'),
    },
  ],
});

export default router;
"###
    .into()
}
