const HOME_URI_PREFIX = "file:///home/";

export function fileUri(file: string) {
  const path = file
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");

  return `${HOME_URI_PREFIX}${path}`;
}

export function pathFromUri(uri: string) {
  if (!uri.startsWith(HOME_URI_PREFIX)) return null;

  const encodedSegments = uri.slice(HOME_URI_PREFIX.length).split("/");
  if (!encodedSegments.length || encodedSegments.some((segment) => !segment)) {
    return null;
  }

  try {
    const segments = encodedSegments.map((segment) => decodeURIComponent(segment));
    if (
      segments.some(
        (segment) =>
          !segment || segment === "." || segment === ".." || segment.includes("/"),
      )
    ) {
      return null;
    }
    return segments.join("/");
  } catch {
    return null;
  }
}

export function sanitizeLspHtml(html: string) {
  const template = document.createElement("template");
  template.innerHTML = html;

  for (const element of template.content.querySelectorAll(
    "script, style, iframe, object, embed, form",
  )) {
    element.remove();
  }

  for (const element of template.content.querySelectorAll("*")) {
    for (const attribute of [...element.attributes]) {
      if (attribute.name.toLowerCase().startsWith("on")) {
        element.removeAttribute(attribute.name);
      }
      if (
        ["href", "src", "xlink:href"].includes(attribute.name.toLowerCase()) &&
        !/^(https?:|mailto:|#)/i.test(attribute.value)
      ) {
        element.removeAttribute(attribute.name);
      }
      if (attribute.name.toLowerCase() === "style") {
        element.removeAttribute(attribute.name);
      }
    }
  }

  return template.innerHTML;
}
