/// <reference path="./.sst/platform/config.d.ts" />
export default $config({
  app(input) {
    return {
      name: "feedrsauros-website",
      home: "cloudflare",
      // Keep the production site if the app is ever removed; throwaway stages clean up after themselves.
      removal: input?.stage === "production" ? "retain" : "remove",
      providers: {
        cloudflare: { package: "@pulumi/cloudflare", version: "6.21.0" },
      },
    };
  },
  async run() {
    const site = new sst.cloudflare.StaticSiteV2("Website", {
      path: "public",
      notFound: "404",
      domain:
        $app.stage === "production"
          ? { name: "feedrsauros.com" }
          : undefined,
    });
    return { url: site.url };
  },
});
