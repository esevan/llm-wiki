// Read-only visual regression checks against the production-rendered DOM.
(() => {
  const visible = node => Boolean(node.getClientRects().length);
  const tabs = Array.from(document.querySelectorAll('.vault-tabs [role="tab"]')).filter(visible).map(node => ({
    id: node.id, selected: node.getAttribute('aria-selected') === 'true',
    background: getComputedStyle(node).backgroundColor,
  }));
  const selectedRows = Array.from(document.querySelectorAll('.vault-browser [aria-current="page"]')).filter(visible).map(node => ({
    control: node.getAttribute('data-control'), classes: node.className,
    background: getComputedStyle(node).backgroundColor, color: getComputedStyle(node).color,
  }));
  const problems = [];
  if (tabs.length) {
    const selected = tabs.filter(tab => tab.selected);
    if (selected.length !== 1) problems.push('Exactly one visible tab must be selected');
    for (const tab of tabs) {
      if (tab.selected && tab.background !== 'rgb(255, 255, 255)') problems.push(tab.id + ' has no selected white surface');
      if (!tab.selected && tab.background === 'rgb(255, 255, 255)') problems.push(tab.id + ' retains the previous selected white surface');
    }
    const id = selected[0]?.id;
    if (id === 'vault-tab-folders' && !document.querySelector('.vault-list .vault-folder')) problems.push('Folders tab does not display the folder tree');
    if (id !== 'vault-tab-folders' && document.querySelector('.vault-list .vault-folder')) problems.push('Folder tree is displayed under a different selected tab');
    const form = document.querySelector('.vault-query');
    if ((id === 'vault-tab-search') !== Boolean(form && visible(form))) problems.push('Search form visibility does not match selected tab');
    if (document.querySelector('.vault-note h1') && selectedRows.length !== 1) problems.push('The displayed note must have one highlighted visible row');
    for (const row of selectedRows) {
      if (!row.classes.includes('is-selected')) problems.push('Selected note has no selection class');
      if (row.background !== 'rgb(255, 240, 246)') problems.push('Selected note has no approved pink surface');
      if (row.color !== 'rgb(185, 24, 96)') problems.push('Selected note has no approved ink color');
    }
  }
  return JSON.stringify({ tabs, selectedRows, problems });
})();
