import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

export const baseOptions: BaseLayoutProps = {
  nav: {
    title: (
      <span className="font-semibold text-lg">Atom</span>
    ),
  },
  links: [
    {
      text: 'GitHub',
      url: 'https://github.com/absmach/atom',
      external: true,
    },
  ],
};
