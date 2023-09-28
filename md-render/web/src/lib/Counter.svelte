<script lang="ts">
  import { render_markdown, MdRenderOpts } from "lib-rs";

  let inputStr = `
# Header
## Subheader

Some text here

`;

  let opts = MdRenderOpts.from_obj({
    with_template: true,
    with_sidenotes: true,
  });

  let rendered = render_markdown(inputStr, opts);
</script>

<p>
  {@html rendered}
</p>
