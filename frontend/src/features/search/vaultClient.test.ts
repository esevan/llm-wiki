import { describe, expect, it } from 'vitest';
import { remarkVaultLinks, resolveNoteLink, type VaultNote } from './vaultClient';
const note=(path:string):VaultNote=>({path,title:path,isMoc:false,managed:false,readable:true,modifiedAt:null});
const notes=[note('Guides/writing.md'),note('MOCs/Guide.md'),note('A/same.md'),note('B/same.md')];
describe('Vault note links',()=>{
  it('resolves sibling, root, unique short, aliased and heading links without guessing ambiguous notes',()=>{
    expect(resolveNoteLink('Guides/writing.md','../MOCs/Guide#Start',notes)).toEqual({path:'MOCs/Guide.md',heading:'Start'});
    expect(resolveNoteLink('MOCs/Guide.md','writing',notes)?.path).toBe('Guides/writing.md');
    expect(resolveNoteLink('Guides/writing.md','#Details',notes)?.path).toBe('Guides/writing.md');
    expect(resolveNoteLink('Guides/writing.md','same',notes)).toBeNull();
  });
  it.each(['../../secret','file:///etc/passwd','javascript:alert(1)','https://example.com/a','/etc/passwd','..\\secret','%2Fetc/passwd','bad%ZZ'])('rejects unsafe or unresolved target %s',target=>{
    expect(resolveNoteLink('Guides/writing.md',target,notes)).toBeNull();
  });
  it('converts wikilinks only in text, preserving fenced/inline code',()=>{
    const tree={type:'root',children:[{type:'paragraph',children:[{type:'text',value:'Read [[Guide#Start|Start here]].'}]},{type:'code',value:'[[literal]]'},{type:'inlineCode',value:'[[literal]]'}]};
    remarkVaultLinks()(tree);
    expect(tree.children[0].children?.[1]).toEqual({type:'link',url:'Guide#Start',children:[{type:'text',value:'Start here'}]});
    expect(tree.children[1].value).toBe('[[literal]]');
  });
});

it('excludes code samples from resolved MOC link counts',async()=>{
 const {noteLinkTargets}=await import('./vaultClient');
 expect(noteLinkTargets('[[Guide]]\n\n```md\n[[Not a link]]\n```\n\n`[[literal]]` and [Real](writing.md)')).toEqual(['Guide','writing.md']);
});

it('groups authored MOC links with their following descriptions without fabricating content',async()=>{
 const {remarkMocRows}=await import('./vaultClient');
 const tree={type:'root',children:[{type:'paragraph',children:[{type:'link',url:'Guide',children:[{type:'text',value:'Guide'}]}]},{type:'paragraph',children:[{type:'text',value:'Authored summary'}]},{type:'heading',children:[{type:'text',value:'Next'}]}]};
 remarkMocRows()(tree);
 expect(tree.children).toHaveLength(2);expect(tree.children[0].type).toBe('vaultMocRow');expect(tree.children[0].children).toHaveLength(2);expect(JSON.stringify(tree)).toContain('Authored summary');
});
