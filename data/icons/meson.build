# Nuntius: own icons (docs/decisions/app-icon.md); upstream's files stay
# in this directory unused.
icon_base_id = 'de.nexumia.Nuntius'

if profile == 'Devel'
    icon_id = icon_base_id + '.Devel'
else
    icon_id = icon_base_id
endif

install_data(
    '@0@.svg'.format(icon_id),
    install_dir: iconsdir / 'hicolor' / 'scalable' / 'apps',
    rename: '@0@.svg'.format(application_id),
)

install_data(
    '@0@-symbolic.svg'.format(icon_base_id),
    install_dir: iconsdir / 'hicolor' / 'symbolic' / 'apps',
    rename: '@0@-symbolic.svg'.format(application_id),
)
