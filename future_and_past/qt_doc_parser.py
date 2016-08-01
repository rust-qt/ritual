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
  def __init__(self):
    ParseException.__init__(self, "Invalid layout")

class NoTypeOriginException(ParseException):
  pass

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
  for suf in ["&&", "**", "*&", "&", "*"]:
    if string.endswith(suf):
      result["indirection"] = suf
      string = string[0:len(string) - len(suf)].strip()
      break

  if string.startswith("const "):
    result["is_const"] = True
    string = string[len("const "):].strip()

  template_match = re.match('^([\w:]+)\s*<(.*)>$', string)
  if template_match:
    string = template_match.group(1).strip()
    result["template_arguments"] = []
    args = template_match.group(2).split(",")
    args = argument_list_template_dirty_fix(args)
    for arg in args:
      arg = arg.strip()
      if arg:
        result["template_arguments"].append(parse_type(arg))

  if "&" in string or "*" in string:
    raise TypeParseException("Invalid type: '%s': too much indirection" % initial_string)

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
    # where arguments don't have a name.
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
        logger.debug("Method is skipped: '%s': typedef encountered" % signature_string)
        continue
      if return_type_string.startswith("virtual"):
        data["virtual"] = True
        return_type_string = return_type_string[len("virtual"):].strip()
      if return_type_string:
        data["return_type"] = parse_type(return_type_string)
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
            data["arguments"].append(arg)
        const_or_pure_suffix = signature_string_parts2[1]
        if const_or_pure_suffix.endswith(" = 0"):
          data["pure_virtual"] = True
          const_or_pure_suffix = const_or_pure_suffix[0:len(const_or_pure_suffix) - len(" = 0")]
        if const_or_pure_suffix.endswith(" const"):
          data["is_const"] = True
          const_or_pure_suffix = const_or_pure_suffix[0:len(const_or_pure_suffix) - len(" const")]
        if const_or_pure_suffix:
          raise ParseException("Unprocessed suffix: '%s'" % const_or_pure_suffix)

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

def parse_inherits(soup):
  h1 = soup.find("h1")
  if not h1:
    raise InvalidLayoutException()
  table = h1.findNext("table", { "class": "alignedsummary" })
  if not table:
    raise InvalidLayoutException()
  for tr in table.findAll("tr"):
    tds = tr.findAll("td")
    if not len(tds) == 2:
      continue
    if tds[0].text.strip() == "Inherits:":
      a = tds[1].find("a")
      if not a:
        return None
      t = parse_type(a.text)
      logger.debug("Found inherits: %s", t)
      return t
  logger.debug("no inherits")
  return None


def parse_nested_types(soup, class_name_or_namespace):
  all_values = {}
  for table in soup.findAll("table", { "class": "valuelist" }):
    if class_name_or_namespace == "QJsonValue":
      h3 = table.findPrevious("h3")
      if h3 and h3.get("id", "") == "toVariant":
        logger.debug("Enum values table is skipped because it is blacklisted.")
        continue

    values = []
    values_dict = {} # for uniqueness check
    for tr in table.findAll("tr"):
      if len(tr.findAll("th")) > 0: continue # skip header
      tds = tr.findAll("td")
      if not len(tds) in [2, 3]: raise InvalidLayoutException()
      value = { "name": tds[0].text.strip(), "value": tds[1].text.strip(), "description": strip_tags(tds[2]) if len(tds) > 2 else "" }
      if class_name_or_namespace:
        if not value["name"].startswith(class_name_or_namespace + "::"):
          raise ParseException("enum item without namespace")
        value["name"] = value["name"][len(class_name_or_namespace + "::"):]
      if value["name"] in values_dict:
        logger.error("Enum value %s is encountered multiple times." % value["name"])
        if value["value"] != values_dict[value["name"]]["value"]:
          logger.error("And values are not the same. Nuff said.")
      else:
        values.append(value)
        values_dict[value["name"]] = value
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
      result["nested_types"] = parse_nested_types(soup, None)
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
    logger.debug("Include file: %s" % result["include_file"])
    full_class_tag = soup.find("span", { "class": "small-subtitle" })
    if full_class_tag:
      result["class"] = full_class_tag.text.strip(" ()")
      result["include_file"] = result["class"].split("::")[0]
      logger.debug("Real include file is believed to be: %s" % result["include_file"])
    else:
      result["class"] = class_without_namespace
    result["nested_types"] = parse_nested_types(soup, result["class"])
    result["nested_types_namespace"] = result["class"]
    result["inherits"] = parse_inherits(soup)

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
      logger.debug("Found typedef: %s" % t)
      if result["class"] == "QTimeZone" and t == "OffsetDataList":
        logger.debug("This typedef is blacklisted here because it is in fact nested.")
      else:
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
    ("QString", { "kind": "class", "name": "QCharRef" }),
    ("QJsonValue", { "kind": "class", "name": "QJsonValueRef" }),
    ("QTextStream", { "kind": "class", "name": "QTextStreamManipulator" }),

  ]:
    if result["include_file"] == include_file:
      result["not_nested_types"] = result.get("not_nested_types", [])
      result["not_nested_types"].append(extra_type)

  for include_file, extra_type in [
    ("QHashIterator", { "kind": "template_type", "name": "Item" }),
    ("QMutableHashIterator", { "kind": "template_type", "name": "Item" }),
    ("QMapIterator", { "kind": "template_type", "name": "Item" }),
    ("QMutableMapIterator", { "kind": "template_type", "name": "Item" }),

    ("QFlags", { "kind": "template_type", "name": "Enum" }),
    ("QFlags", { "kind": "template_type", "name": "Zero" }),

    ("QSharedPointer", { "kind": "template_type", "name": "Deleter" }),
    ("QVarLengthArray", { "kind": "template_type", "name": "Prealloc" }),
    ("QVarLengthArray", { "kind": "template_type", "name": "Prealloc1" }),
    ("QVarLengthArray", { "kind": "template_type", "name": "Prealloc2" }),
    ("QPair", { "kind": "template_type", "name": "TT1" }),
    ("QPair", { "kind": "template_type", "name": "TT2" }),
    ("QHash", { "kind": "template_type", "name": "InputIterator" }),

    ("QVariant", { "kind": "enum", "name": "Type" }), #TODO: add enum values
  ]:
    if result["include_file"] == include_file:
      result["nested_types"] = result.get("nested_types", [])
      result["nested_types"].append(extra_type)


  return result


def parse(input_folder):
  headers_data = []
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
      headers_data.append(result)
    except ParseException as e:
      logger.error("Parse error: %s: %s" % (filename, e.message))
  return headers_data


def known_basic_types():
  types_data = {}
  def add_known_type(name, origin):
    if name in types_data:
      logger.warning("Type data is overwritten for %s", name)
    types_data[name] = { "origin": origin }

  for t in [
    "void", "float", "double", "bool",
    #"qint8", "quint8", "qint16", "quint16", "qint32", "quint32", "qint64", "quint64",
    #"qlonglong","qulonglong", "qreal", "quintptr", "qintptr", "qptrdiff",
    "char", "signed char", "unsigned char",
    "short", "signed short", "unsigned short",
    "int", "signed int", "unsigned int",
    "long", "signed long", "unsigned long",
    "long long int", "unsigned long long int",
    "wchar_t", "size_t"
  ]:
    add_known_type(t, "c_built_in")

  for t in ["CFDataRef", "CFURLRef", "NSData", "NSString", "CFStringRef", "NSURL", "NSDate", "CFDateRef"]:
    add_known_type(t, "mac_os_native")

  for t in ["GUID", "HANDLE"]:
    add_known_type(t, "windows_native")

  for t in [
    "va_list", "FILE",
    "std::string", "std::u16string", "std::u32string", "std::list", "std::wstring", "std::initializer_list",
    "std::pair", "std::map", "std::vector"
  ]:
    add_known_type(t, "cpp_std")

  for t in ["T", "T1", "T2", "X", "Key", "ForwardIterator", "Container", "Cleanup"]:
    add_known_type(t, "template_argument")

  for t in ["PointerToMemberFunction", "MemberFunction", "MemberFunctionOk", "UnaryFunction", "Functor", "QtCleanUpFunction"]:
    add_known_type(t, "function_pointer")

  for t in ["QWidget"]: #TODO: change when QtWidgets support comes
    add_known_type(t, "fake")

  for t in ["QVersionNumber"]: #TODO: change this when updating to Qt 5.6
    add_known_type(t, "fake")

  for t in ["QMap<Key, T>::const_iterator", "QMap<Key, T>::iterator", "QHash<Key, T>::const_iterator",
            "QHash<Key, T>::iterator", "QMap<Key,  T>::const_iterator"]:
    #TODO: do something with these types
    add_known_type(t, "fake")

  return types_data


def add_qt_types(headers_data, types_data):
  def doc_page_exists_for_class(name):
    for header_data in headers_data:
      if header_data.get("class", "") == name:
        return True
    return False

  for header_data in headers_data:
    if "class" in header_data:
      type_data = {"kind": "class", "origin": "qt", "qt_header": header_data["include_file"] }
      if header_data.get("inherits", None):
        type_data["inherits"] = header_data["inherits"]
      name = header_data["class"]
      if name in types_data:
        logger.warning("Type data is overwritten for %s", name)
      types_data[name] = type_data
    for t in header_data.get("nested_types", []):
      type_data = t.copy()
      type_data["origin"] = "qt"
      type_data["qt_header"] = header_data["include_file"]
      name = type_data.pop("name")
      if "nested_types_namespace" in header_data:
        name = header_data["nested_types_namespace"] + "::" + name
        if "enum" in type_data:
          type_data["enum"] = header_data["nested_types_namespace"] + "::" + type_data["enum"]
      if doc_page_exists_for_class(name):
        logger.debug("Data for nested type (%s) is not added because it has separate doc page", name)
      else:
        if name in types_data:
          logger.warning("Type data is overwritten for %s", name)
        types_data[name] = type_data
    for t in header_data.get("not_nested_types", []):
      type_data = t.copy()
      name = type_data.pop("name")
      type_data["origin"] = "qt"
      type_data["qt_header"] = header_data["include_file"]
      if name in types_data:
        logger.warning("Type data is overwritten for %s", name)
      types_data[name] = type_data
    header_data.pop("nested_types_namespace", None)
    header_data.pop("nested_types", None)
    header_data.pop("not_nested_types", None)

def add_typedef_data(types_data):
  def check_type(t):
    if not t["base"] in types_data:
      raise ParseException("Unknown type: %s" % t["base"])
    for arg in t.get("template_arguments", []):
      check_type(arg)

  def add_meaning(name, meaning):
    meaning_parsed = parse_type(meaning)
    if not name in types_data:
      raise ParseException("Unknown type: %s" % name)
    check_type(meaning_parsed)
    if not meaning_parsed["base"] in types_data:
      raise ParseException("Unknown type: %s" % meaning)
    types_data[name]["meaning"] = meaning_parsed

  # these types are not used anywhere, so
  # we should just forget about them
  bad_names = []
  for name, data in types_data.iteritems():
    if data.get("kind", None) == "typedef":
      if name.endswith("::ConstIterator") or \
         name.endswith("::Iterator") or \
         name.endswith("::iterator_category"):
        bad_names.append(name)

  for name in bad_names:
    types_data.pop(name)



  add_meaning("qint8", "signed char")
  add_meaning("quint8", "unsigned char")
  add_meaning("qint16", "signed short")
  add_meaning("quint16", "unsigned short")
  add_meaning("qint32", "signed int")
  add_meaning("quint32", "unsigned int")
  add_meaning("qint64", "long long int") # todo: __int64 on Windows
  add_meaning("quint64", "unsigned long long int") # todo: unsigned __int64 on Windows
  add_meaning("qlonglong", "long long int") # todo: __int64 on Windows
  add_meaning("qulonglong", "unsigned long long int") # todo: unsigned __int64 on Windows
  add_meaning("qintptr", "long long int") # todo: can be qint64 or qint32
  add_meaning("quintptr", "unsigned long long int") # todo: can be quint64 or quint32
  add_meaning("qptrdiff", "long long int") # todo: can be qint64 or qint32
  add_meaning("QList::difference_type", "long long int") # todo: can be qint64 or qint32
  add_meaning("qreal", "double")
  add_meaning("uchar", "unsigned char")
  add_meaning("uint", "unsigned int")
  add_meaning("ulong", "unsigned long")
  add_meaning("ushort", "unsigned short")

  add_meaning("Qt::HANDLE", "void*")

  add_meaning("QByteArray::const_iterator", "const char*")
  add_meaning("QByteArray::iterator", "char*")
  add_meaning("QString::const_iterator", "const QChar*")
  add_meaning("QString::iterator", "QChar*")

  add_meaning("QFileInfoList", "QList<QFileInfo>")
  add_meaning("QModelIndexList", "QList<QModelIndex>")
  add_meaning("QObjectList", "QList<QObject>")
  add_meaning("QTimeZone::OffsetDataList", "QList<QTimeZone::OffsetData>")
  add_meaning("QVariantHash", "QHash<QString, QVariant>")
  add_meaning("QVariantMap", "QMap<QString, QVariant>")
  add_meaning("QVariantList", "QList<QVariant>")

  add_meaning("QVariantAnimation::KeyValues", "QVector<QPair<qreal, QVariant>>")
  add_meaning("QXmlStreamEntityDeclarations", "QVector<QXmlStreamEntityDeclaration>")
  add_meaning("QXmlStreamNamespaceDeclarations", "QVector<QXmlStreamNamespaceDeclaration>")
  add_meaning("QXmlStreamNotationDeclarations", "QVector<QXmlStreamNotationDeclaration>")

  # these types can't be automatically processed
  blacklist = [
    "QFunctionPointer",
    "QGlobalStatic::Type",
    "QEasingCurve::EasingFunction",
    "QLoggingCategory::CategoryFilter",
    "QMessageLogger::CategoryFunction",
    "QSettings::ReadFunc",
    "QSettings::WriteFunc",
    "QVarLengthArray::const_iterator",
    "QVarLengthArray::iterator",
    "QVector::const_iterator",
    "QVector::const_reference",
    "QVector::iterator",
    "QVector::reference",
    "QtMessageHandler"
  ]

  unknown_typedefs = []
  for name, data in types_data.iteritems():
    if data.get("kind", None) == "typedef":
      if not "meaning" in data:
        if not name in blacklist:
          unknown_typedefs.append(name)
  if unknown_typedefs:
    unknown_typedefs.sort()
    logger.warning("Unknown typedefs: \n%s", "\n".join(unknown_typedefs))

  #types_data["QVariant::Type"]["values"] = []
  #for value in types_data["QMetaType::Type"]["values"]:
  #  new_value = value.copy()
  #  if new_value["name"].startswith("Q"):
  #    new_value["name"] = new_value["name"][1:]
  #    types_data["QVariant::Type"]["values"].append(new_value)
  types_data["QVariant::Type"] = { "origin": "fake" }
  types_data["QEvent::Type"]["values"] = [
    v for v in types_data["QEvent::Type"]["values"]
    if v["name"] != "EnterEditFocus" and v["name"] != "LeaveEditFocus"
  ]


def fix_method_types(headers_data, types_data):
  used_types = set()
  def fix_nested_types(t, current_namespace):
    for subtype in t.get("template_arguments", []):
      fix_nested_types(subtype, current_namespace)

    if t["base"] == "QFile::Permissions":
      # upcasting everything is too much for me!
      t["base"] = "QFileDevice::Permissions"
    #print "test1 ", current_namespace, t
    namespace_parts = []
    while True:
      if current_namespace:
        namespace_parts = current_namespace.split("::")
      #print "test2 ", namespace_parts
      for i in sorted(range(1+len(namespace_parts)), reverse=True):
        candidate = "::".join(namespace_parts[:i] + [t["base"]])
        #print "test3 candidate ", candidate
        if candidate in types_data:
          t["base"] = candidate
          used_types.add(candidate)
          return
      if current_namespace and current_namespace in types_data:
        n = types_data[current_namespace]["inherits"]
        if n:
          logger.debug("Switching namespace from %s to %s", current_namespace, n["base"])
          current_namespace = n["base"]
        else:
          break
      else:
        break

    raise NoTypeOriginException("Unknown type: %s" % t["base"])


  for header_data in headers_data:
    current_namespace = None
    methods = header_data.get("methods", [])
    logger.debug("Checking include file: " + header_data["include_file"])
    if "class" in header_data:
      current_namespace = header_data["class"]
      #logger.warning("Class: " + header_data["class"])
      if not methods:
        logger.warning("Class %s doesn't have any methods" % header_data["class"])

    if header_data.get("inherits", None):
      fix_nested_types(header_data["inherits"], current_namespace)

    #logger.warning("Namespace: " + unicode(current_namespace))
    for m in methods:
      try:
        if "return_type" in m:
          fix_nested_types(m["return_type"], current_namespace)
        for arg in m.get("arguments", []):
          fix_nested_types(arg["type"], current_namespace)
      except NoTypeOriginException as e:
        logger.warning("%s (#include <%s>)\n%s\n" % (e.message, header_data["include_file"], pp.pformat(m)))

  bad_names = []
  for name, data in types_data.iteritems():
    if data.get("kind", "") == "typedef" and not name in used_types:
      logger.info("Removing unused typedef: %s", name)
      bad_names.append(name)
  for name in bad_names:
    types_data.pop(name)

def process(input_folder):
  headers_data = parse(input_folder)
  types_data = known_basic_types()
  add_qt_types(headers_data, types_data)
  fix_method_types(headers_data, types_data)
  add_typedef_data(types_data)
  return { "headers_data": headers_data, "type_info": types_data }

if len(sys.argv) < 3:
  print "Usage: parser doc_html_folder output_filename"
else:
  logger.info("Parsing documentation...")
  parse_result = process(sys.argv[1])
  logger.info("Writing JSON result...")
  with open(sys.argv[2], "w") as f:
    json.dump(parse_result, f, indent=2, sort_keys=True)

  logger.info("Done.")
