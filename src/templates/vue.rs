use super::{styles::styles, ProjectProps, Template};
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
    let mut content = HashMap::from([
        ("src/main.ts", Content::Str(main())),
        ("src/routes.ts", Content::Str(routes())),
        ("src/images/logo.svg", Content::Str(logo("Vue".into()))),
        ("src/ui/index.ts", Content::Str(ui_index())),
        ("src/ui/components/Button.vue", Content::Str(button())),
        ("src/ui/components/Navbar.vue", Content::Str(navbar())),
    ]);
    content.extend(styles());
    content
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

fn logo(name: String) -> String {
    format!(
        r###"<svg version="1.1" viewBox="0 0 261.76 226.69" xmlns="http://www.w3.org/2000/svg">
  <title>{}</title>
  <g transform="matrix(1.3333 0 0 -1.3333 -76.311 313.34)">
    <g transform="translate(178.06 235.01)">
      <path d="m0 0-22.669-39.264-22.669 39.264h-75.491l98.16-170.02 98.16 170.02z" fill="#41b883"/>
    </g>
    <g transform="translate(178.06 235.01)">
      <path d="m0 0-22.669-39.264-22.669 39.264h-36.227l58.896-102.01 58.896 102.01z" fill="#34495e"/>
    </g>
  </g>
</svg>
"###,
        name
    )
}

fn ui_index() -> String {
    r###"import './styles/index.scss';
import Button from './components/Button.vue';
import Navbar from './components/Navbar.vue';

export default {
  install(app: any) {
    app.component('button-el', Button);
    app.component('navbar-el', Navbar);
  },
};
"###
    .into()
}

fn button() -> String {
    r###"<template>
  <button class="btn">
    <slot></slot>
  </button>
</template>

<style scoped lang="scss">
.btn {
  background-image: linear-gradient(to top, var(--teal) 0%, var(--blue) 51%, var(--teal) 100%);
  padding: .75rem .5rem;
  text-align: center;
  text-transform: none;
  transition: 0.5s;
  background-size: auto 200%;
  color: var(--white);
  border-radius: 0.5rem;
  display: flex;
  border-color: transparent;
  font-weight: 400;
  font-size: 0.8rem;
  cursor: pointer;
  line-height: 0.5rem;
  align-items: center;
  height: min-content;

  &[size=l] {
    text-transform: uppercase;
    font-weight: 700;
    font-size: 1.8rem;
  }

  &:hover {
    background-position: right center;
    /* change the direction of the change here */
    color: var(--white);
    text-decoration: none;
  }
}
</style>  
"###
    .into()
}

fn navbar() -> String {
    r###"<template>
  <div class="navbar__container">
    <div class="navbar__menu">
      <slot></slot>
    </div>
  </div>
</template>

<style lang="scss">
.navbar {
  &__container {
    position: absolute;
    width: 100%;
  }

  &__menu {
    display: flex;
    justify-content: center;
    padding: 1rem;
    gap: 1rem;

    a {
      color: var(--white);
      font-weight: 700;
      text-decoration: none;
      transition: 1s;
      text-transform: uppercase;

      &:hover {
        color: var(--purple);
      }
    }

    input, textarea, select {
      padding: .45rem .5rem;
      border-color: transparent;
      border-radius: 0.5rem;
      font-size: 0.8rem;
      line-height: 0.5rem;
      height: min-content;
      color: var(--black);
    }
  }
}</style>
"###
    .into()
}
