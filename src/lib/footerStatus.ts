/** The right-hand part of the popup/panel status bar (spec 004 FR-034). */
export interface FooterStatus {
  text: string;
  /** Makes the status a button, e.g. the panel's auto-translate mode that cycles on click. */
  onclick?: () => void;
  title?: string;
  /** An optional key shown before the text, e.g. Esc. */
  key?: string;
}
