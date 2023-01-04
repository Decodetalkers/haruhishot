project('haruhishot', 'rust',
  version: '0.2.3',
  meson_version: '>= 0.60'
)

dependency('wayland-client')
dependency('wlroots')

cargo = find_program('cargo', version: '>= 1.65')

rustc = find_program('rustc', version: '>= 1.65')

if get_option('debug')
  if get_option('enable-notify')
    command = [
      cargo, 'build', '--features=notify',
      '&&',
      'cp', meson.global_source_root() / 'target' / 'debug' / meson.project_name(), '@OUTPUT@'
    ]
  else
    command = [
      cargo, 'build',
      '&&',
      'cp', meson.global_source_root() / 'target' / 'debug' / meson.project_name(), '@OUTPUT@'
    ]
  endif
else
  if get_option('enable-notify')
    command = [
      cargo, 'build', '--features=notify',
      '--release',
      '&&',
      'cp', meson.global_source_root() / 'target' / 'release' / meson.project_name(), '@OUTPUT@'
    ]
  else
    command = [
      cargo, 'build',
      '--release',
      '&&',
      'cp', meson.global_source_root() / 'target' / 'release' / meson.project_name(), '@OUTPUT@'
    ]
  endif
endif

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
