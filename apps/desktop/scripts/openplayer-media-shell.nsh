!define OP_THUMBNAIL_HANDLER_GUID "{e357fccd-a995-4576-b01f-234630154e96}"
!define OP_LEGACY_IMAGE_HANDLER_GUID "{BB2E617C-0920-11D1-9A0B-00C04FC2D6C1}"
!define OP_PROPERTY_THUMBNAIL_HANDLER_CLSID "{9DBD2C50-62AD-11D0-B806-00C04FD706EC}"

!macro OP_REGISTER_VIDEO_THUMBNAIL EXT MIME
  WriteRegStr HKCU "Software\Classes\.${EXT}" "PerceivedType" "video"
  WriteRegStr HKCU "Software\Classes\.${EXT}" "Content Type" "${MIME}"
  WriteRegStr HKCU "Software\Classes\.${EXT}\ShellEx\${OP_THUMBNAIL_HANDLER_GUID}" "" "${OP_PROPERTY_THUMBNAIL_HANDLER_CLSID}"
  WriteRegStr HKCU "Software\Classes\.${EXT}\ShellEx\${OP_LEGACY_IMAGE_HANDLER_GUID}" "" "${OP_PROPERTY_THUMBNAIL_HANDLER_CLSID}"
!macroend

!macro OP_REGISTER_AUDIO_METADATA EXT MIME
  WriteRegStr HKCU "Software\Classes\.${EXT}" "PerceivedType" "audio"
  WriteRegStr HKCU "Software\Classes\.${EXT}" "Content Type" "${MIME}"
  WriteRegStr HKCU "Software\Classes\.${EXT}\ShellEx\${OP_THUMBNAIL_HANDLER_GUID}" "" "${OP_PROPERTY_THUMBNAIL_HANDLER_CLSID}"
  WriteRegStr HKCU "Software\Classes\.${EXT}\ShellEx\${OP_LEGACY_IMAGE_HANDLER_GUID}" "" "${OP_PROPERTY_THUMBNAIL_HANDLER_CLSID}"
!macroend

!macro OP_REGISTER_MEDIA_SHELL_METADATA
  WriteRegDWORD HKCU "Software\Classes\SystemFileAssociations\video" "Treatment" 3
  WriteRegDWORD HKCU "Software\Classes\SystemFileAssociations\video" "ThumbnailCutoff" 1

  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "3gp" "video/3gpp"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "avi" "video/avi"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "flv" "video/x-flv"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "m2ts" "video/vnd.dlna.mpeg-tts"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "m4v" "video/mp4"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "mkv" "video/x-matroska"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "mov" "video/quicktime"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "mp4" "video/mp4"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "mpeg" "video/mpeg"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "mpg" "video/mpeg"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "ogv" "video/ogg"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "webm" "video/webm"
  !insertmacro OP_REGISTER_VIDEO_THUMBNAIL "wmv" "video/x-ms-wmv"

  !insertmacro OP_REGISTER_AUDIO_METADATA "aac" "audio/aac"
  !insertmacro OP_REGISTER_AUDIO_METADATA "flac" "audio/flac"
  !insertmacro OP_REGISTER_AUDIO_METADATA "m4a" "audio/mp4"
  !insertmacro OP_REGISTER_AUDIO_METADATA "mp3" "audio/mpeg"
  !insertmacro OP_REGISTER_AUDIO_METADATA "oga" "audio/ogg"
  !insertmacro OP_REGISTER_AUDIO_METADATA "ogg" "audio/ogg"
  !insertmacro OP_REGISTER_AUDIO_METADATA "opus" "audio/ogg"
  !insertmacro OP_REGISTER_AUDIO_METADATA "wav" "audio/wav"
!macroend
