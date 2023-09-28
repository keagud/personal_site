import { render_markdown, MdRenderOpts, process_sidenotes } from "lib-rs";


const submitButton = <HTMLButtonElement>document.getElementById("md-submit-button")!;




function renderMarkdown() {

  const opts = MdRenderOpts.from_obj({ with_template: true, with_sidenotes:  true });

  console.log(opts.with_sidenotes);

  const inputElement = <HTMLTextAreaElement>document.getElementById('md-input-area')!;


  const outputElement = <HTMLTextAreaElement>document.getElementById('md-output-container')!;


  let mdInput = inputElement.value;


  if (opts.with_sidenotes) {
    console.log(mdInput)

  }

  const rendered = render_markdown(mdInput, opts);
  outputElement.innerHTML = rendered;

}


submitButton.addEventListener("click", (_) => { renderMarkdown() });

