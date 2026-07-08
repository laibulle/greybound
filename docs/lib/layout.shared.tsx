import type { BaseLayoutProps } from "fumadocs-ui/layouts/shared";

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <>
          <img
            alt=""
            aria-hidden="true"
            src="/greybound-robine-mark.svg"
            style={{ height: 28, width: 28 }}
          />
          <span>Greybound</span>
        </>
      ),
    },
  };
}
