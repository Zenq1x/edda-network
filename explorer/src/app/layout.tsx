import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'Edda Explorer',
  description: 'Real-time block explorer for the Edda Network',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
