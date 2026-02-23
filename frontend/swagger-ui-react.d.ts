declare module "swagger-ui-react" {
  import type { ComponentType } from "react";
  interface SwaggerUIProps {
    spec?: object | string;
    url?: string;
    [key: string]: unknown;
  }
  const SwaggerUI: ComponentType<SwaggerUIProps>;
  export default SwaggerUI;
}
