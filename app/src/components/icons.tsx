// 線のアイコン。16px の枠に 1.75px の線で描き、文字の色を継ぐ

export function ChevronRight() {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.75"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d="m6.5 4.5 3.5 3.5-3.5 3.5" />
    </svg>
  )
}

export function Cross() {
  return (
    <svg
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.75"
      stroke-linecap="round"
      aria-hidden="true"
    >
      <path d="m4.5 4.5 7 7m0-7-7 7" />
    </svg>
  )
}
