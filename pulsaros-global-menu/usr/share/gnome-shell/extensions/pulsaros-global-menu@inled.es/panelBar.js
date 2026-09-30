// Placement of the menu-bar buttons in the panel.
//
// All three menu sources put their buttons in one contiguous run at the left
// of the panel, app-name button first. The run cannot be addressed by
// absolute panel index: any extension that inserts a left-box button ahead of
// ours shifts the whole run right, while fixed indices keep inserting at the
// old spot, which is how the app name once ended up after the menus. Every
// insert is measured from where the run currently sits instead.

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

/**
 * Panel index the run starts at: the app-name button when it exists,
 * otherwise the first free slot after the workspace indicator.
 *
 * Anchoring on the app-name button also heals a run that is already out of
 * order, since menu buttons are placed after wherever it currently is.
 */
function runStart() {
	// _leftBox is private, but nothing public reports where a panel button
	// currently sits, and an absolute index is exactly what breaks here
	const children = Main.panel._leftBox.get_children();
	const apple = children.findIndex(c => c.first_child?._pulsarAppleLogo);
	if (apple !== -1)
		return apple + 1;
	const app = children.findIndex(c => c.first_child?._globalMenuAppName);
	if (app !== -1)
		return app;
	const ours = children.findIndex(c => c.first_child?._globalMenuButton);
	if (ours !== -1)
		return ours;
	const activities = Main.panel.statusArea.activities?.container;
	const at = activities ? children.indexOf(activities) : -1;
	return at + 1;
}

/**
 * Add a menu-bar button to the panel. `rank` is its place within the run:
 * 0 is the app-name button, menu buttons start at 1.
 */
export function addBarButton(role, btn, rank) {
	Main.panel.addToStatusArea(role, btn, runStart() + rank, 'left');
}
