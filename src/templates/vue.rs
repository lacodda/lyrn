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
        ("src/components/App.vue", Content::Str(container_component())),
        ("src/components/Home.vue", Content::Str(home_page())),
        ("src/components/About.vue", Content::Str(component_page("About"))),
        ("src/components/Info.vue", Content::Str(component_page("Info"))),
        ("src/images/logo.svg", Content::Str(logo("Vue"))),
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

fn component_page(name: &str) -> String {
    let name_lc = name.to_lowercase();
    format!(
        r###"<template>
  <div class="{}__container">
    <div class="{}__title">
      <h1>{}</h1>
    </div>
  </div>
</template>

<style scoped lang="scss">
.{} {{
  &__container {{
    display: grid;
    justify-content: center;
    align-content: center;
    background: var(--gr-lime-blue);
    width: 100%;
    height: 100%;
  }}
  &__title {{
    display: flex;
    h1 {{
      font-size: var(--font-size-h1);
      width: max-content;
      text-transform: uppercase;
      background: var(--teal);
      background: var(--gr-teal-blue);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
    }}
  }}
}}
</style>
"###,
        name_lc, name_lc, name, name_lc
    )
}

fn home_page() -> String {
    r###"<template>
  <div class="home__container">
    <div class="home__title">
      <img :src="require('images/logo.svg')" width="100" />
      <h1>Vue 3 Boilerplate</h1>
    </div>
    <div class="home__counter">
      <button-el size="l" @click="inc">+</button-el>
      <button-el size="l" @click="dec">−</button-el>
      <div class="home__count">{{count}}</div>
    </div>
  </div>
</template>

<script lang="ts">
import { Ref, ref } from 'vue';

export default {
  setup () {
    const count: Ref<number> = ref(0);

    function inc (): void {
      count.value++;
    }
    function dec (): void {
      count.value--;
    }
    return { count, inc, dec };
  },
};
</script>

<style scoped lang="scss">
.home {
  &__container {
    display: grid;
    justify-content: center;
    align-content: center;
    background: var(--gr-azure-pink);
    width: 100%;
    height: 100%;
  }
  &__title {
    display: flex;
    column-gap: 1rem;
    h1 {
      font-size: var(--font-size-h1);
      width: max-content;
      text-transform: uppercase;
      background: var(--teal);
      background: var(--gr-teal-blue);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
    }
  }
  &__counter {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
  }
  &__count {
    font-size: 2rem;
    font-weight: 700;
    color: var(--white);
  }
}
</style>
"###
    .into()
}

fn container_component() -> String {
    r###"<template>
  <div class="app">
    <navbar-el>
      <router-link to="/">home</router-link>
      <router-link to="/info">info</router-link>
      <router-link to="/about">about</router-link>
    </navbar-el>
    <router-view />
  </div>
</template>

<style lang="scss">
.app {
  width: 100%;
  height: 100%;
}
</style>
"###
    .into()
}

fn logo(name: &str) -> String {
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
