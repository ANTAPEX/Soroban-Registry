import type { StorybookConfig } from "@storybook/nextjs-vite";

const config: StorybookConfig = {
  // The scaffold's own ../stories directory is gone, and this glob never
  // covered the project's stories, which live beside the components they
  // document. Storybook was loading the demo and nothing else.
  stories: [
    "../components/**/*.mdx",
    "../components/**/*.stories.@(js|jsx|mjs|ts|tsx)",
  ],
  addons: [
    "@chromatic-com/storybook",
    "@storybook/addon-vitest",
    "@storybook/addon-a11y",
    "@storybook/addon-docs",
    "@storybook/addon-onboarding",
  ],
  framework: "@storybook/nextjs-vite",
  staticDirs: ["../public"],
};
export default config;
