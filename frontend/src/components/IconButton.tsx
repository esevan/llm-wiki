import type { ButtonHTMLAttributes, PropsWithChildren } from 'react';

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  kind?: 'primary' | 'tiny' | 'tiny hot';
  labelVisible?: boolean;
}

export function IconButton({ children, label, kind = 'tiny', labelVisible = false, ...props }: PropsWithChildren<IconButtonProps>) {
  return (
    <button className={`${kind} icon-action${labelVisible ? ' labelled' : ''}`} data-tooltip={label} aria-label={label} {...props}>
      <span aria-hidden="true">{children}</span>{labelVisible && <span>{label}</span>}
    </button>
  );
}
