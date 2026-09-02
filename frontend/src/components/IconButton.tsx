import type { ButtonHTMLAttributes, PropsWithChildren } from 'react';

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  kind?: 'primary' | 'tiny' | 'tiny hot';
}

export function IconButton({ children, label, kind = 'tiny', ...props }: PropsWithChildren<IconButtonProps>) {
  return (
    <button className={`${kind} icon-action`} data-tooltip={label} aria-label={label} {...props}>
      {children}
    </button>
  );
}
