use crate::libs::types::Content;
use std::collections::HashMap;

pub fn styles() -> HashMap<&'static str, Content> {
    HashMap::from([
        ("src/ui/styles/index.scss", Content::Str(style_index())),
        ("src/ui/styles/_reset.scss", Content::Str(style_reset())),
        ("src/ui/styles/_variables.scss", Content::Str(style_variables())),
        ("src/ui/styles/_scaffolding.scss", Content::Str(style_scaffolding())),
    ])
}

fn style_index() -> String {
    r###"@import url('https://fonts.googleapis.com/css2?family=Noto+Sans:wght@400;700&display=swap');

@import 'reset';
@import 'variables';
@import 'scaffolding';
"###
    .into()
}

fn style_reset() -> String {
    r###"/*
  1. Use a more-intuitive box-sizing model.
*/
*,
*::before,
*::after {
  box-sizing: border-box;
}

/*
    2. Remove default margin
  */
* {
  margin: 0;
}

/*
    3. Allow percentage-based heights in the application
  */
html,
body {
  height: 100%;
}

/*
    Typographic tweaks!
    4. Add accessible line-height
    5. Improve text rendering
  */
body {
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
}

/*
    6. Improve media defaults
  */
img,
picture,
video,
canvas,
svg {
  display: block;
  max-width: 100%;
}

/*
    7. Remove built-in form typography styles
  */
input,
button,
textarea,
select {
  font: inherit;
}

/*
    8. Avoid text overflows
  */
p,
h1,
h2,
h3,
h4,
h5,
h6 {
  overflow-wrap: break-word;
}

/*
    9. Create a root stacking context
  */
#root,
#__next {
  isolation: isolate;
}
"###
    .into()
}

fn style_variables() -> String {
    r###":root {
    --black: #343434;
    --gray: #dedede;
    --blue: #06347e;
    --white: #ffffff;
    --teal: #008080;
    --azure: #31CDDD;
    --fuxia: #F700F7;
    --pink: #ec62b5;
    --violet: #9333ea;
    --purple: #38006b;
    --lime: #86efac;
    --gr-teal-blue: linear-gradient(to right, var(--teal) 0%, var(--blue) 100%);
    --gr-teal-pink: linear-gradient(to right, var(--teal) 0%, var(--pink) 100%);
    --gr-azure-fuxia: linear-gradient(to right, var(--azure) 0%, var(--fuxia) 100%);
    --gr-violet-purple: linear-gradient(to right, var(--violet) 0%, var(--purple) 100%);
    --gr-azure-pink: linear-gradient(var(--azure), var(--pink));
    --gr-lime-blue: linear-gradient(var(--lime), var(--blue));
    --gr-pink-violet: linear-gradient(var(--pink), var(--violet));
    --font-family: 'Noto Sans', sans-serif;
    --font-size: 20px;
    --font-size-h1: 2.6rem;
    --font-size-h2: 1.6rem;
    --font-size-h3: 1.1rem;
}
"###
    .into()
}

fn style_scaffolding() -> String {
    r###"html {
  font-size: var(--font-size);
  font-family: var(--font-family);
  color: var(--black);
}

body {
  & > :not(noscript) {
    display: contents;
  }
}
"###
    .into()
}
