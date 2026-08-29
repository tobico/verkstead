import { defineConfig, type Plugin } from "vite";
import solid from "vite-plugin-solid";

/// The share build: the same sources as the viewer, built to one HTML file with
/// every byte it needs inside it.
///
/// What comes out of here is a template rather than a page — the server puts a
/// Conversation into the slot `share.html` leaves and hands the result over as a
/// download (`crates/server/src/sharing.rs`). It is a second config rather than a
/// mode of the viewer's because almost everything about it is the opposite: one
/// entry, one chunk, nothing hashed, nothing copied in beside it, and nothing
/// left pointing at a URL.
///
/// The rule the whole file exists for: **a share makes no request to anything.**
/// It is read from a disk, out of an email, through somebody else's viewer page,
/// and every one of those is a place where a fetch either fails or tells a host
/// who is reading. So the script and the stylesheets are inlined here, the fonts
/// stay the system stack they already were, and the icons are Font Awesome's
/// path data compiled into the bundle rather than a sheet to load.
export default defineConfig({
  plugins: [solid(), oneFile()],

  // The service worker, the manifest and the icons are the *app's*, and a share
  // is not one. Nothing here is copied in beside the document, because a
  // document with something beside it is not one file.
  publicDir: false,

  build: {
    outDir: "dist-share",
    emptyOutDir: true,

    // The share document rather than the app's.
    rollupOptions: {
      input: "share.html",
      output: {
        // One chunk. Splitting exists so a browser can fetch the parts it needs
        // when it needs them, and there is nothing here to fetch with.
        inlineDynamicImports: true,
        entryFileNames: "share.js",
        assetFileNames: "share.[ext]",

        // And a plain script rather than a module. Nothing here imports
        // anything at run time — there is one chunk and it is inside the
        // document — so the module semantics buy nothing, and what they cost is
        // a page opened from a disk: a `file://` document has a null origin, and
        // a module is the one kind of script that has an opinion about that.
        format: "iife",
      },
    },

    // One stylesheet, for the reason there is one chunk.
    cssCodeSplit: false,

    // Everything an asset could be, as a data URL: what is left over otherwise
    // is a file beside the document, and there is to be nothing beside the
    // document.
    assetsInlineLimit: Number.MAX_SAFE_INTEGER,

    // And no preload hints, which are links to files that will not exist by the
    // time [`oneFile`] has finished.
    modulePreload: false,
  },
});

/// Fold the built chunk and stylesheet into the document, and leave nothing else
/// behind.
///
/// vite writes a page that *links* what it built, which is right for a site and
/// wrong for a file somebody emails. This runs after everything — `post`, so the
/// bundle is final — swaps each link for what it pointed at, and drops the files
/// it inlined so that the output directory is the one document and no more.
///
/// It refuses rather than degrades. A build that quietly wrote a share still
/// pointing at `/assets/` would be a file that looks right on the machine that
/// made it and is blank everywhere else, which is the one failure nobody would
/// catch in time.
function oneFile(): Plugin {
  return {
    name: "verkstead-share-one-file",
    enforce: "post",

    generateBundle(_options, bundle) {
      const document = Object.values(bundle).find(
        (output) => output.type === "asset" && output.fileName.endsWith(".html"),
      );

      if (document === undefined || document.type !== "asset") {
        throw new Error("the share build produced no document to inline into");
      }

      let html = String(document.source);

      for (const output of Object.values(bundle)) {
        if (output === document) {
          continue;
        }

        if (output.type === "chunk") {
          html = swap(html, script(output.fileName), () => inlineScript(output.code));
        } else if (output.fileName.endsWith(".css")) {
          html = swap(html, sheet(output.fileName), () => `<style>${String(output.source)}</style>`);
        } else {
          throw new Error(
            `the share build wrote ${output.fileName}, which cannot go inside the document`,
          );
        }

        delete bundle[output.fileName];
      }

      // Nothing may be left asking for anything. `/assets/` is where vite writes
      // what it names by content, so a mention of it here is a link this plugin
      // failed to fold in.
      if (html.includes("/assets/")) {
        throw new Error("the share document still points at a file beside it");
      }

      document.source = html;
    },
  };
}

/// The tag vite writes for a built chunk, matched by the file it names.
function script(file: string): RegExp {
  return new RegExp(`<script[^>]*src="[^"]*${quoted(file)}"[^>]*></script>`);
}

/// And the one it writes for the stylesheet.
function sheet(file: string): RegExp {
  return new RegExp(`<link[^>]*href="[^"]*${quoted(file)}"[^>]*>`);
}

/// A file name as a pattern may hold it: every character that means something to
/// a regular expression, spelled as itself.
function quoted(file: string): string {
  return file.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/// One swap, refusing where there was nothing to swap: a file in the bundle that
/// the document never linked is a file this plugin is about to drop and
/// something is about to want.
///
/// The replacement arrives as a function rather than as a string because the
/// string is compiled JavaScript: `$&` and its relatives mean something to
/// `replace`, and a bundle is certain to contain one eventually.
function swap(html: string, tag: RegExp, put: () => string): string {
  if (!tag.test(html)) {
    throw new Error(`the share document links nothing matching ${String(tag)}`);
  }

  return html.replace(tag, put);
}

/// The chunk as it may be written inside a `<script>`.
///
/// A plain script with nothing on it. vite hoists the tag it wrote into the
/// head, and `defer` means nothing on an inline script — so what waits for the
/// document is the boot itself, in `src/share/index.tsx`, which is the one place
/// that can be sure of it.
///
/// One substitution, and it is the same hazard the record's own JSON has: a
/// `</script` anywhere in the compiled code — in a string, in a regular
/// expression — would end the block early. The escape means the same thing to
/// JavaScript and nothing to the parser looking for the end of the tag.
function inlineScript(code: string): string {
  return `<script>${code.replace(/<\/script/gi, "<\\/script")}</script>`;
}
