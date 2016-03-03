#!/usr/bin/env python
from BeautifulSoup import BeautifulSoup, NavigableString
import re
import pprint
import glob
import sys
import json
import pprint
import logging
import subprocess
import os

pp = pprint.PrettyPrinter()

def color_red(prt): return ("\033[91m {}\033[00m" .format(prt))
def color_green(prt): return ("\033[92m {}\033[00m" .format(prt))
def color_yellow(prt): return ("\033[93m {}\033[00m" .format(prt))
def color_light_purple(prt): return ("\033[94m {}\033[00m" .format(prt))
def color_purple(prt): return ("\033[95m {}\033[00m" .format(prt))
def color_cyan(prt): return ("\033[96m {}\033[00m" .format(prt))
def color_light_gray(prt): return ("\033[97m {}\033[00m" .format(prt))
def color_black(prt): return ("\033[98m {}\033[00m" .format(prt))

class ColorLogger:
  def __init__(self, real_logger):
    self.real_logger = real_logger

  def debug(self, text, *args, **kwargs):
    text = color_light_purple(text)
    apply(self.real_logger.debug, (text,) + args, kwargs)

  def debug_pp(self, obj):
    self.debug(pp.pformat(obj))

  def info(self, text, *args, **kwargs):
    text = color_green(text)
    apply(self.real_logger.info, (text,) + args, kwargs)

  def warning(self, text, *args, **kwargs):
    text = color_red(text)
    apply(self.real_logger.warning, (text,) + args, kwargs)

  def error(self, text, *args, **kwargs):
    text = color_red(text)
    apply(self.real_logger.error, (text,) + args, kwargs)

  def critical(self, text, *args, **kwargs):
    text = color_red(text)
    apply(self.real_logger.critical, (text,) + args, kwargs)


real_logger = logging.getLogger("qt_doc_parser")
logging_level = logging.INFO
stream_handler = logging.StreamHandler(sys.stderr)
stream_handler.setLevel(logging_level)
real_logger.setLevel(logging_level)
real_logger.addHandler(stream_handler)

logger = ColorLogger(real_logger)

class ParseException(Exception):
  pass


class TypeParseException(ParseException):
  pass


class InvalidLayoutException(ParseException):
  pass

class NoTypeOriginException(ParseException):
  pass


def fix_nested_types(class_name, t, nested_types):
  def is_nested(name):
    for t in nested_types:
      if t["name"] == name:
        return True
    return False

  if is_nested(t["base"]):
    t["base"] = class_name + "::" + t["base"]


# soup.text sometimes deletes spaces between words.
# this is a space-preserving alternative
def strip_tags(soup):
  return u' '.join(soup.findAll(text=True))


def parse_type(string):
  initial_string = string
  logger.debug("parse_type %s" % string)
  string = string.strip()
  if not string:
    raise ParseException("Type is missing")
  result = {}
  if string.endswith("*"):
    result["pointer"] = True
    string = string[:-1].strip() # no, it's not a smile
  elif string.endswith("&&"):
    result["double_reference"] = True
    string = string[:-2].strip()
  elif string.endswith("&"):
    result["reference"] = True
    string = string[:-1].strip()
  if string.startswith("const "):
    result["const"] = True
    string = string[len("const "):].strip()

  template_match = re.match('^([\w:]+)\s*<(.*)>$', string)
  if template_match:
    result["template"] = True
    string = template_match.group(1).strip()
    result["template_arguments"] = []
    for arg in template_match.group(2).split(","):
      arg = arg.strip()
      if arg:
        result["template_arguments"].append(parse_type(arg))

  if "&" in string or "*" in string:
    raise TypeParseException("Invalid type: '%s': double indirection" % initial_string)

  result["base"] = string

  logger.debug("parse_type result: %s" % unicode(result))
  return result


def argument_list_template_dirty_fix(arg_list):
  while True:
    error_found = False
    for i in range(0, len(arg_list)):
      if arg_list[i].count('<') != arg_list[i].count('>'):
        if i + 1 < len(arg_list):
          #print arg_list
          #print i
          r = []
          if i > 0:
            r += arg_list[:i]
          r.append(arg_list[i]+", " + arg_list[i+1])
          if i + 2 < len(arg_list):
            r += arg_list[i+2:]
          arg_list = r
          #print arg_list
          error_found = True
          break
    if not error_found: break
  return arg_list


def parse_argument(string):
  logger.debug("parse_argument %s" % string)
  if string in ["int", "bool"]:
    # there are a few places in the documentation
    # where arguments don't have a name

    # TODO: this is probably better to detect using lack of space character
    return { "type": parse_type(string), "name": "value" }

  result = {}
  parts1 = string.split('=')
  if len(parts1) != 1 and len(parts1) != 2:
    raise InvalidLayoutException()
  if len(parts1) == 2:
    result["default_value"] = parts1[1].strip()
  other_part = parts1[0].strip()
  re1 = re.findall("\\w+$", other_part)
  if len(re1) != 1: raise InvalidLayoutException()
  result["name"] = re1[0]
  type_string = other_part[0:len(other_part)-len(result["name"])]
  result["type"] = parse_type(type_string)
  return result


def parse_methods_for_typedefs(table):
  result = []
  for row in table.findAll("tr"):
    tds = row.findAll("td")
    if len(tds) != 2: raise InvalidLayoutException()
    #print "test 3 ", tds
    return_type_string = tds[0].text.strip()
    signature_string = strip_tags(tds[1]).strip()
    #print "test 4 ", return_type_string
    if return_type_string == "typedef":
      result.append(signature_string)
  return result

def parse_methods(table, class_name, section_attrs, nested_types):
  result = []
  for row in table.findAll("tr"):
    tds = row.findAll("td")
    if len(tds) != 2: raise InvalidLayoutException()
    return_type_string = tds[0].text.strip()
    signature_string = strip_tags(tds[1]).strip()
    try:
      data = section_attrs.copy()
      if class_name:
        data["scope"] = "class"
      else:
        data["scope"] = "global"
      if return_type_string == "typedef":
        logger.warning("Method is skipped: '%s': typedef encountered" % signature_string)
        continue
      if return_type_string.startswith("virtual"):
        data["virtual"] = True
        return_type_string = return_type_string[len("virtual"):].strip()
      if return_type_string:
        data["return_type"] = parse_type(return_type_string)
        if class_name:
          fix_nested_types(class_name, data["return_type"], nested_types)
      if re.match("^\\w+$", signature_string):
        data["name"] = signature_string
        data["variable"] = True
        data["type"] = data["return_type"]
        data.pop("return_type")
      else:
        if signature_string.startswith("operator()"):
          name_end_index = signature_string.find("(", len("operator()"))
        else:
          name_end_index = signature_string.find("(")
          if name_end_index < 0: raise Exception("Invalid signature syntax")
        data["name"] = signature_string[0:name_end_index].strip()
        signature_string_parts2 = signature_string[name_end_index+1:].strip().rsplit(")", 1)
        if len(signature_string_parts2) != 2: raise ParseException("Invalid signature syntax")
        data["arguments"] = []
        arg_list = signature_string_parts2[0].strip().split(',')
        arg_list = argument_list_template_dirty_fix(arg_list)
        for part in arg_list:
          if not part: continue
          part = part.strip()
          if part == "...":
            data["variable_arguments"] = True
          else:
            arg = parse_argument(part)
            if class_name and arg["type"]:
              fix_nested_types(class_name, arg["type"], nested_types)
            data["arguments"].append(arg)
        if signature_string_parts2[1].strip() == "const":
          data["const"] = True

        if data["name"].startswith("operator"):
          data["operator"] = data["name"][len("operator"):].strip()
        if class_name:
          if data["name"] == class_name:
            data["constructor"] = True
            if "return_type" in data:
              raise ParseException("Constructors are not allowed to have return types")
          elif data["name"] == "~" + class_name:
            data["destructor"] = True
            if "return_type" in data:
              raise ParseException("Destructors are not allowed to have return types")
          elif "operator" in data:
            pass
            # operators may or may not have return type
          else: # not constructor
            if not "return_type" in data:
              raise ParseException("No return type in a method")

      result.append(data)
    except TypeParseException as e:
      logger.warning("Method is skipped: '%s': %s" % (signature_string, e.message))

  return result



def parse_section(soup, class_name, id, attrs, nested_types):
  header = soup.find("h2", id=id)
  if not header: return []
  return parse_methods(header.findNext("table"), class_name, attrs, nested_types)

def parse_section_for_typedefs(soup, id):
  header = soup.find("h2", id=id)
  #print "test 1"
  if not header: return []
  #print "test 2"
  return parse_methods_for_typedefs(header.findNext("table"))



def parse_macros(soup, id):
  macros_header = soup.find("h2", id=id)
  if not macros_header: return None
  macros = []
  for row in macros_header.findNext("table").findAll("tr"):
    macros.append(row.text)
  return macros

def parse_nested_types(soup, class_name_or_namespace):
  all_values = {}
  for table in soup.findAll("table", { "class": "valuelist" }):
    if class_name_or_namespace == "QJsonValue":
      h3 = table.findPrevious("h3")
      if h3 and h3.get("id", "") == "toVariant":
        logger.warning("Enum values table is skipped because it is blacklisted.")
        continue

    values = []
    for tr in table.findAll("tr"):
      if len(tr.findAll("th")) > 0: continue # skip header
      tds = tr.findAll("td")
      if not len(tds) in [2, 3]: raise Exception("Unknown HTML layout")
      value = { "name": tds[0].text.strip(), "value": tds[1].text.strip(), "description": strip_tags(tds[2]) if len(tds) > 2 else "" }
      if not value["name"].startswith(class_name_or_namespace + "::"):
        raise ParseException("enum item without namespace")
      value["name"] = value["name"][len(class_name_or_namespace + "::"):]
      values.append(value)
    current_pos = table
    found_name = None
    while current_pos:
      current_pos = current_pos.findPrevious("a")
      if current_pos and current_pos.get("name") and current_pos["name"].endswith("-enum"):
        found_name = current_pos["name"]
        break
    if not found_name:
      raise ParseException("Can't find anchor for values table!")
    all_values[found_name] = all_values.get(found_name, []) +  values

  all_types = []
  link_href_to_enum = {}

  types_header = soup.find("h2", { "id": "Typesx" })
  if not types_header:
    types_header = soup.find("h2", { "id": "public-types" })
  if not types_header:
    types_header = soup.find("h2", { "id": "types" })
  if not types_header:
    logger.debug("No types header")
    return []
  for row in types_header.findNext("table").findAll("tr"):
    tds = row.findAll("td")
    if len(tds) != 2: raise ParseException("Unknown HTML layout")
    kind_of_type = tds[0].text
    if kind_of_type == "enum":
      link = tds[1].find("a")
      if not link: raise ParseException("enum without link")
      if not link.get("href"): raise ParseException("enum link without href")
      if not "#" in link["href"]: raise ParseException("enum link href without anchor")
      anchor = link["href"].split("#")[1]
      name = tds[1].text.split("{")[0].strip()
      if not anchor in all_values:
        raise ParseException("values table not found for enum")
      all_types.append({
        "kind": "enum",
        "name": name,
        "values": all_values[anchor]})
      link_href_to_enum[link["href"]] = name

  for row in types_header.findNext("table").findAll("tr"):
    tds = row.findAll("td")
    if len(tds) != 2: raise ParseException("Unknown HTML layout")
    kind_of_type = tds[0].text
    if kind_of_type == "enum": continue
    name = tds[1].text.strip()
    if kind_of_type == "flags":
      link = tds[1].find("a")
      if not link: raise ParseException("flags without link")
      if not link.get("href"): raise ParseException("flags link without href")
      if not link["href"] in link_href_to_enum:
        raise ParseException("no enum found for flags")
      enum = link_href_to_enum[link["href"]]
      all_types.append({
        "kind": "flags",
        "name": name,
        "enum": enum})
    else:
      all_types.append({
        "kind": kind_of_type,
        "name": name})

  if class_name_or_namespace == "QByteArray":
    # they forgot to document this!
    all_types.append({ "kind": "typedef", "name": "iterator" })
    all_types.append({ "kind": "typedef", "name": "const_iterator" })

  logger.debug("nested types found: %s" % pp.pformat(all_types))

  return all_types


def parse_doc(filename):
  logger.info("Parsing " + filename)
  data = open(filename,'r').read()
  soup = BeautifulSoup(data, convertEntities=BeautifulSoup.HTML_ENTITIES)
  result = {}
  title = soup.find("h1", { "class": "title" })
  if not title: raise InvalidLayoutException()


  if title.text.startswith("<Qt") or title.text == "Qt Namespace":
    result["type"] = "global"
    macros = parse_macros(soup, "Macrosx")
    if macros: result["macros"] = macros
    if title.text == "Qt Namespace":
      result["include_file"] = "Qt"
      result["nested_types"] = parse_nested_types(soup, "Qt")
      result["nested_types_namespace"] = "Qt"
    else:
      end_index = title.text.find(">")
      if not end_index:
        raise ParseException("Invalid global header title")
      result["include_file"] = title.text[1:end_index]

    logger.info("Include file: %s" % result["include_file"])
    result["methods"] = parse_section(soup, None, "Functions", {}, result.get("nested_types", []))
    return result

  title_parts = title.text.split(' ')
  if len(title_parts) == 2 and title_parts[1].strip() == "Class":
    result["type"] = "class"
    class_without_namespace = title_parts[0].strip()
    result["include_file"] = class_without_namespace
    logger.info("Include file: %s" % result["include_file"])
    full_class_tag = soup.find("span", { "class": "small-subtitle" })
    if full_class_tag:
      result["class"] = full_class_tag.text.strip(" ()")
    else:
      result["class"] = class_without_namespace
    result["nested_types"] = parse_nested_types(soup, result["class"])
    result["nested_types_namespace"] = result["class"]

    if not result["class"].startswith("Q"):
      print "Warning: %s: class %s doesn't start with Q" % (filename, result["class"])
  else:
    subprocess.call(["xdg-open", filename])
    raise ParseException("Unsupported title value in %s" % filename)

  result["methods"] = []

  found_typedefs = parse_section_for_typedefs(soup, "related-non-members")
  if found_typedefs:
    result["not_nested_types"] = []
    for t in found_typedefs:
      logger.info("Found typedef: %s" % t)
      result["not_nested_types"].append({ "kind": "typedef", "name": t })

  for section_name, attrs in [
    ("public-functions", {}),
    ("protected-functions", { "protected": True }),
    ("public-slots", { "slot": True }),
    ("protected-slots", { "slot": True, "protected": True }),
    ("static-public-members", { "static": True }),
    ("static-protected-members", { "static": True, "protected": True }),
    ("signals", { "signal": True }),
    ("related-non-members", {})
  ]:
    cl = None if section_name == "related-non-members" else class_without_namespace
    result["methods"] += parse_section(soup, cl, section_name, attrs, result["nested_types"])
  macros = parse_macros(soup, "macros")
  if macros:
    result["macros"] = macros


  for include_file, extra_type in [
    ("QByteArray", { "kind": "class", "name": "QByteRef" }),
    ("QBitArray", { "kind": "class", "name": "QBitRef" }),
    ("QJsonValue", { "kind": "class", "name": "QJsonValueRef" }),
    ("QHashIterator", { "kind": "typedef", "name": "Item" })
  ]:
    if result["include_file"] == include_file:
      result["not_nested_types"] = result.get("not_nested_types", [])
      result["not_nested_types"].append(extra_type)

  return result


def parse(input_folder):
  all_data = []
  # some files don't contain anything useful
  bad_endings = "members", "obsolete", "compat", "example", "pro", "cpp", "h", "ui"

  bad_files = "codec-big5", "codec-euckr", "codec-eucjp", "codec-gbk", \
              "codec-tscii", "codec-big5hkscs", "codecs-jis", "codec-sjis", \
              "signalsandslots", "events", "animation", "object", "containers", "io",\
              "plugins", "eventsandfilters", "statemachine-api", "qtcore-module", "properties",\
              "implicit-sharing", "animation-overview", "resources", "datastreamformat", \
              "timers", "shared", "custom-types", "qtcore-index", "statemachine", "io-functions", \
              "objecttrees", "json", "metaobjects"

  for filename in glob.iglob(input_folder + '/*.html'):
    basename = os.path.basename(filename)
    bad = False
    for ending in bad_endings:
      if basename.endswith("-%s.html" % ending):
        bad = True
        break
    for f in bad_files:
      if basename == ("%s.html" % f):
        bad = True
        break
    if bad:
      logger.debug("File is skipped because it is blacklisted: %s" % filename)
      continue

    try:
      result = parse_doc(filename)
      all_data.append(result)
    except ParseException as e:
      logger.error("Parse error: %s: %s" % (filename, e.message))

  type_origins = {}

  for t in [
    "void", "float", "double", "bool",
    "qint8", "quint8", "qint16", "quint16", "qint32", "quint32", "qint64", "quint64",
    "qlonglong","qulonglong", "qreal", "quintptr", "qintptr", "qptrdiff",
    "char", "signed char", "unsigned char", "uchar",
    "short", "signed short", "unsigned short", "ushort",
    "int", "signed int", "unsigned int", "uint",
    "long", "signed long", "unsigned long", "ulong",
    "wchar_t"
  ]:
    type_origins[t] = "primitive"

  for t in ["CFDataRef", "CFURLRef", "NSData", "NSString", "CFStringRef", "NSURL", "NSDate", "CFDateRef"]:
    type_origins[t] = "mac_os_native"

  for t in ["std::string", "std::u16string", "std::u32string", "std::list", "std::wstring", "std::initializer_list"]:
    type_origins[t] = "cpp_std"

  for t in ["T", "T1", "T2", "X", "Key"]:
    type_origins[t] = "template_argument"

  for header_data in all_data:
    if "class" in header_data:
      type_origins[ header_data["class"] ] = {"qt_header": header_data["include_file"] }
    for t in header_data.get("nested_types", []):
      name = header_data["nested_types_namespace"] + "::" + t["name"]
      type_origins[name] = {"qt_header": header_data["include_file"] }
    for t in header_data.get("not_nested_types", []):
      name = t["name"]
      type_origins[name] = {"qt_header": header_data["include_file"] }


  def check_type_recursive(t):
    if not t["base"] in type_origins:
      raise NoTypeOriginException("Unknown type origin: '%s'" % t["base"])
    for tt in t.get("template_arguments", []):
      check_type_recursive(tt)


  for header_data in all_data:
    methods = header_data.get("methods", [])
    if "class" in header_data and not methods:
      logger.warning("Class %s doesn't have any methods" % header_data["class"])
    for m in methods:
      try:
        if "return_type" in m:
          check_type_recursive(m["return_type"])
        for arg in m.get("arguments", []):
          check_type_recursive(arg["type"])
      except NoTypeOriginException as e:
        logger.warning("%s (#include <%s>)\n%s\n" % (e.message, header_data["include_file"], pp.pformat(m)))


  return all_data

if len(sys.argv) < 3:
  print "Usage: parser doc_html_folder output_filename"
else:
  logger.info("Parsing documentation...")
  parse_result = parse(sys.argv[1])
  logger.info("Writing JSON result...")
  with open(sys.argv[2], "w") as f:
    json.dump(parse_result, f, indent=2)

  logger.info("Done.")
