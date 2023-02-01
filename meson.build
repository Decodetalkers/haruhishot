project('haruhishot', 'rust',
  version: '0.3.2',
  meson_version: '>= 0.60'
)

dependency('wayland-client')
dependency('wlroots')

cargo = find_program('cargo', version: '>= 1.66')

rustc = find_program('rustc', version: '>= 1.66')

command = [ cargo , 'build']

if get_option('enable-notify')
  command += ['--features', 'notify']
endif

if get_option('enable-gui')
  command += ['--features', 'gui']
endif

targetdir = 'debug'

if not get_option('debug')
  command += '--release'
  targetdir = 'release'
endif

command += [
  '&&',
  'cp',
  meson.global_source_root() / 'target' / targetdir / meson.project_name(),
  '@OUTPUT@'
]

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
icondir = datadir / 'pixmaps'

haruhishot_target = custom_target(meson.project_name(),
  output: meson.project_name(),
  build_by_default: true,
  install: true,
  install_dir: bindir,
  console: true,
  command: command
)

if get_option('enable-notify')
  install_data('misc/haruhi_failed.png',
    install_dir: icondir
  )
  install_data('misc/haruhi_successed.png',
    install_dir: icondir
  )
endif

if get_option('man-pages')
  manpage = get_option('mandir')
  scdoc = dependency(
    'scdoc',
    version: '>= 1.9.7',
    native: true
  )

  if scdoc.found()
    custom_target(
      'haruhishot.1',
      input: 'scdoc/haruishot.1.scd',
      output: 'haruhishot.1',
      command: scdoc.get_variable('scdoc'),
      feed: true,
      capture: true,
      install: true,
      install_dir: prefix / manpage / 'man1'
    )
  endif
endif

# Install desktop entry
if get_option('desktop-entry')
  install_data('misc/haruhishot.desktop',
    install_dir: datadir / 'applications'
  )
  install_data('misc/haruhishot.svg',
    install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
  )
endif

