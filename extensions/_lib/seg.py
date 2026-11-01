# extensions/_lib/seg.py：出站消息段构造器，宿主注册为 loquat_seg。
# 内核配套库，非插件（无签名、不进责任链）。
#
# v2 插件 handle() 直接返回段列表即一条消息：
#     import loquat_seg as seg
#     return [seg.text("看这个"), seg.image("https://…"), seg.reply(ctx["message_id"])]
#
# 段为 protojson 形态（adapter.proto 的 Segment oneof，键名 lowerCamel），与入站
# ctx["segments"] 同构；宿主原样序列化下发，各平台适配器负责翻成本地格式。
# 列表保持扁平，不嵌套（嵌套归适配器，不归框架）。

import base64


def text(s) -> dict:
    return {"text": {"text": str(s)}}


def at(target) -> dict:
    """at 某人；target 为平台用户 ID，"all" 表示全体（支持的平台）。"""
    return {"at": {"target": str(target)}}


def reply(message_id) -> dict:
    """引用回复；message_id 一般取 ctx["message_id"]。"""
    return {"reply": {"messageId": str(message_id)}}


def face(face_id) -> dict:
    """平台表情；ID 跨平台不保证一致。"""
    return {"face": {"faceId": str(face_id)}}


def image(url=None, *, file_id=None, inline=None) -> dict:
    return {"image": _media(url, file_id, inline)}


def voice(url=None, *, file_id=None, inline=None) -> dict:
    return {"voice": _media(url, file_id, inline)}


def video(url=None, *, file_id=None, inline=None) -> dict:
    return {"video": _media(url, file_id, inline)}


def file(url=None, *, file_id=None, inline=None, filename="") -> dict:
    body = _media(url, file_id, inline)
    if filename:
        body["filename"] = str(filename)
    return {"file": body}


def extension(namespace, kind, data=None) -> dict:
    """平台私有段：namespace 为平台名（如 "napcat"），kind 为平台内段类型。
    适配器只认自己的 namespace，其他平台会报无法表达。"""
    return {"extension": {"namespace": str(namespace), "type": str(kind), "data": dict(data or {})}}


def _media(url, file_id, inline) -> dict:
    """媒体来源三选一：url / file_id / inline（bytes，按 protojson 规则转 base64）。
    url 可为 http(s) 或 file://（本地文件由适配器所在机器解析）。"""
    if url:
        return {"url": str(url)}
    if file_id:
        return {"fileId": str(file_id)}
    if inline is not None:
        return {"inline": base64.b64encode(bytes(inline)).decode("ascii")}
    raise ValueError("媒体段需要 url / file_id / inline 之一")
