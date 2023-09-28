import './app.css'
import App from './App.svelte'

import * as foo from "lib-rs";


const init = async () => {

  await import("lib-rs");

}

const app = new App({
  target: document.getElementById('app')!,
})

export default app
