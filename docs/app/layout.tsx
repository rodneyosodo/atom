import type { ReactNode } from 'react';
import { RootProvider } from 'fumadocs-ui/provider';
import type { Metadata } from 'next';
import 'fumadocs-ui/style.css';
import './global.css';

export const metadata: Metadata = {
  title: {
    template: '%s | Atom',
    default: 'Atom Docs',
  },
  description: 'Identity and Authorization service for IoT and cloud-native systems',
};

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}
