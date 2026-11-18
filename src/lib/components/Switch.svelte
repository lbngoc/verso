<script lang="ts">
  // On/off control used everywhere a checkbox used to be (spec 009 FR-037).
  let {
    checked,
    onchange,
    label,
    disabled = false,
  }: {
    checked: boolean;
    onchange: (checked: boolean) => void;
    label: string;
    disabled?: boolean;
  } = $props();
</script>

<label class="switch" class:disabled>
  <input
    type="checkbox"
    role="switch"
    {checked}
    {disabled}
    aria-label={label}
    onchange={(event) => onchange(event.currentTarget.checked)}
  />
  <span class="track" aria-hidden="true"></span>
</label>

<style>
  .switch {
    position: relative;
    display: inline-flex;
    width: var(--switch-width, 32px);
    height: var(--switch-height, 18px);
    flex: none;
    cursor: pointer;
  }

  .switch.disabled {
    opacity: 0.5;
    cursor: default;
  }

  input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: inherit;
  }

  .track {
    position: absolute;
    inset: 0;
    border-radius: calc(var(--switch-height, 18px) / 2);
    background: var(--separator);
    transition: background 0.15s ease;
    pointer-events: none;
  }

  .track::after {
    content: "";
    position: absolute;
    width: var(--switch-thumb, 14px);
    height: var(--switch-thumb, 14px);
    top: calc((var(--switch-height, 18px) - var(--switch-thumb, 14px)) / 2);
    left: calc((var(--switch-height, 18px) - var(--switch-thumb, 14px)) / 2);
    border-radius: 50%;
    background: var(--accent-contrast);
    box-shadow: var(--shadow-control);
    transition: transform 0.15s ease;
  }

  input:checked + .track {
    background: var(--accent);
  }

  input:checked + .track::after {
    transform: translateX(calc(var(--switch-width, 32px) - var(--switch-height, 18px)));
  }

  input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
