project('haruhishot', 'rust',
  version: '0.2.9',
  meson_version: '>= 0.60'
)

dependency('wayland-client')
dependency('wlroots')

cargo = find_program('cargo', version: '>= 1.65')

rustc = find_program('rustc', version: '>= 1.65')

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
