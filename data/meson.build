subdir('icons')

gnome = import('gnome')

configure_file(
  input: 'org.dupot.sambasharemanager.in',
  output: 'org.dupot.sambasharemanager.policy',
  configuration: conf,
  install_dir: datadir / 'polkit-1' / 'actions'
)

configure_file(
  input: 'samba-share.desktop.in',
  output: 'samba-share.desktop',
  configuration: conf,
  install_dir: datadir / 'applications'
)