/**
 * UI preferences that persist across navigation within a session.
 * These survive component unmount/remount — they live at module scope.
 */

// Clip review preferences
let autoPlay_ = $state(false);
let autoAdvance_ = $state(false);
let showRender_ = $state(false);
let showDetails_ = $state(false);
let showMediaInfo_ = $state(false);

export function getAutoPlay(): boolean { return autoPlay_; }
export function setAutoPlay(v: boolean) { autoPlay_ = v; }

export function getAutoAdvance(): boolean { return autoAdvance_; }
export function setAutoAdvance(v: boolean) { autoAdvance_ = v; }

export function getShowRender(): boolean { return showRender_; }
export function setShowRender(v: boolean) { showRender_ = v; }

export function getShowDetails(): boolean { return showDetails_; }
export function setShowDetails(v: boolean) { showDetails_ = v; }

export function getShowMediaInfo(): boolean { return showMediaInfo_; }
export function setShowMediaInfo(v: boolean) { showMediaInfo_ = v; }

// Game view filter preferences
let selectedSegment_ = $state<number | null>(null);
let selectedEventType_ = $state<string | null>(null);
let eventsExpanded_ = $state(true);
let rendersExpanded_ = $state(true);
let metadataExpanded_ = $state(false);
let livestreamsExpanded_ = $state(false);
let stateInfoExpanded_ = $state(false);

export function getSelectedSegment(): number | null { return selectedSegment_; }
export function setSelectedSegment(v: number | null) { selectedSegment_ = v; }

export function getSelectedEventType(): string | null { return selectedEventType_; }
export function setSelectedEventType(v: string | null) { selectedEventType_ = v; }

export function getEventsExpanded(): boolean { return eventsExpanded_; }
export function setEventsExpanded(v: boolean) { eventsExpanded_ = v; }

export function getRendersExpanded(): boolean { return rendersExpanded_; }
export function setRendersExpanded(v: boolean) { rendersExpanded_ = v; }

export function getMetadataExpanded(): boolean { return metadataExpanded_; }
export function setMetadataExpanded(v: boolean) { metadataExpanded_ = v; }

export function getLivestreamsExpanded(): boolean { return livestreamsExpanded_; }
export function setLivestreamsExpanded(v: boolean) { livestreamsExpanded_ = v; }

export function getStateInfoExpanded(): boolean { return stateInfoExpanded_; }
export function setStateInfoExpanded(v: boolean) { stateInfoExpanded_ = v; }
