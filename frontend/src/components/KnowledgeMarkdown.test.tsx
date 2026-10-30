import { render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { KnowledgeMarkdown } from './KnowledgeMarkdown';
import { vaultClient } from '../features/search/vaultClient';
vi.mock('mermaid',()=>({default:{initialize:vi.fn()}}));
vi.mock('../features/search/vaultClient',async original=>({...await original<typeof import('../features/search/vaultClient')>(),vaultClient:{image:vi.fn().mockResolvedValue({dataUrl:'data:image/png;base64,cG5n'})}}));
it('renders original images beside draft prose without exposing remote URLs',async()=>{
 const ref='llm-wiki-image:'+'b'.repeat(64);
 render(<KnowledgeMarkdown>{`The diagram explains the decision.\n\n![Decision map](${ref})\n\n![External](https://outside.test/track.png)`}</KnowledgeMarkdown>);
 await screen.findByRole('img',{name:'Decision map'});
 expect(vaultClient.image).toHaveBeenCalledWith('',ref);
 expect(screen.getByText('The diagram explains the decision.')).toBeVisible();
 expect(document.querySelector('img[src^="https:"]')).toBeNull();
});
