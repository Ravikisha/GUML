/**
 * The GUML mark.
 *
 * Inline rather than an `<img src="/logo-mark.svg">` for one reason that matters and one that follows
 * from it: the bars are `currentColor`, so the mark tracks the page's text colour in either theme with
 * no second file and no flash while an image loads. `assets/logo-light.svg` and `logo-dark.svg` exist
 * for contexts that strip inheritance — GitHub's markdown renderer — and are generated from the same
 * source, so there is one drawing rather than three that can drift.
 *
 * What it means: fluid on the left, structured on the right. A model's output arrives amorphous and
 * leaves as a laid-out interface, and the necks reaching rightward are the compiler. One bar stays
 * accent-coloured because that is where the transition happens.
 */
export function Logo({ className }: { className?: string }) {
  return (
    <svg
      // The same box as `assets/logo-mark.svg`: cropped to the drawing with a small even margin. The
      // original 1240 square left a third of the frame empty, which made the mark read as smaller than
      // the space it took at every size it is actually used.
      viewBox="200 212 820 756"
      className={className}
      role="img"
      aria-label="GUML"
      // The filter needs ids that cannot collide with another inline SVG on the page.
      focusable="false"
    >
      <defs>
        {/* Metaball goo: blur the circles together, then push alpha through a steep curve so the soft
            overlap snaps back to a hard edge. The necks are real intersections, not guessed paths. */}
        <filter
          id="guml-goo"
          x="-30%"
          y="-30%"
          width="160%"
          height="160%"
          colorInterpolationFilters="sRGB"
        >
          <feGaussianBlur in="SourceGraphic" stdDeviation="20" result="blur" />
          <feColorMatrix
            in="blur"
            type="matrix"
            values="1 0 0 0 0 0 1 0 0 0 0 0 1 0 0 0 0 0 24 -11"
            result="goo"
          />
        </filter>
        <mask id="guml-notch">
          <rect width="1240" height="1240" fill="#fff" />
          <circle cx="600" cy="596" r="44" fill="#000" />
        </mask>
      </defs>

      <g fill="#f73b20" mask="url(#guml-notch)">
        <g filter="url(#guml-goo)">
          <circle cx="528" cy="322" r="96" />
          <circle cx="452" cy="412" r="76" />
          <circle cx="392" cy="506" r="86" />
          <circle cx="458" cy="600" r="60" />
          <circle cx="412" cy="692" r="76" />
          <circle cx="348" cy="790" r="128" />
          <circle cx="502" cy="848" r="68" />
          <circle cx="556" cy="894" r="54" />
          <circle cx="604" cy="498" r="52" />
        </g>
        {/* Droplets that have not resolved into structure yet. Far enough out that the goo threshold
            leaves them whole. */}
        <circle cx="360" cy="330" r="44" />
        <circle cx="290" cy="578" r="44" />
      </g>

      <g fill="currentColor">
        <rect x="645" y="252" width="355" height="140" rx="52" />
        <rect x="868" y="424" width="132" height="146" rx="44" />
        <rect x="625" y="607" width="375" height="151" rx="50" />
        <rect x="620" y="788" width="120" height="152" rx="42" />
        <rect x="765" y="788" width="235" height="152" rx="46" />
      </g>

      {/* Drawn outside the filter: the goo threshold pulls a filtered edge inward, and this contact is
          the whole point of the mark. */}
      <rect x="596" y="470" width="80" height="56" rx="28" fill="#f73b20" />
      <rect x="645" y="424" width="210" height="146" rx="48" fill="#f73b20" />
    </svg>
  );
}
