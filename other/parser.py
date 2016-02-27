#!/usr/bin/env python
from BeautifulSoup import BeautifulSoup, NavigableString
import re
import pprint
import glob
import sys
import json

def strip_tags(soup):
  return u' '.join(soup.findAll(text=True))


def parse_type(string):
  string = string.strip()
  if not string:
    raise Exception("Type is missing")
  result = {}
  if "<T>" in string:
    result["template"] = True
  if string.endswith("*"):
    result["pointer"] = True
    string = string[:-1].strip() # no, it's not a smile
  elif string.endswith("&&"):
    print "Double reference type encountered!"
    return False
  elif string.endswith("&"):
    result["reference"] = True
    string = string[:-1].strip()
  if string.startswith("const"):
    result["const"] = True
    string = string[len("const"):].strip()
  result["base"] = string
  if result["base"] == "T":
    result["template"] = True
  return result



def parse_argument(string):
  # dirty hacks section!
  if string in ["int", "bool"]:
    return { "type": parse_type(string), "name": "value" }

  # end of dirty hacks section

  if string == "...":
    return { "variable_arguments": True }
#  print "test ", string
  result = {}
  parts1 = string.split('=')
  if len(parts1) != 1 and len(parts1) != 2: raise Exception("Invalid argument syntax")
  if len(parts1) == 2:
    result["default_value"] = parts1[1].strip()
  other_part = parts1[0].strip()
  re1 = re.findall("\\w+$", other_part)
  if len(re1) != 1: raise Exception("Invalid argument syntax")
  result["name"] = re1[0]
  type_string = other_part[0:len(other_part)-len(result["name"])]
  result["type"] = parse_type(type_string)
  if not result["type"]:
    return False
  return result


def parse_methods(table, class_name, section_attrs):
  result = []
  for row in table.findAll("tr"):
    tds = row.findAll("td")
    if len(tds) != 2: raise Exception("Unknown HTML layout")
    return_type_string = tds[0].text.strip()
    signature_string = strip_tags(tds[1]).strip()
    data = section_attrs.copy()
    if class_name:
      data["scope"] = "class"
    else:
      data["scope"] = "global"
    if return_type_string == "typedef":
      continue
    if return_type_string.startswith("virtual"):
      data["virtual"] = True
      return_type_string = return_type_string[len("virtual"):].strip()
    if return_type_string:
      data["return_type"] = parse_type(return_type_string)
      if not data["return_type"]:
        print "Unsupported type encountered. Method is skipped:\n%s\n" % signature_string
        continue
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
      if len(signature_string_parts2) != 2: raise Exception("Invalid signature syntax")
      data["arguments"] = []
      argument_failed = False
      for part in signature_string_parts2[0].strip().split(','):
        if not part: continue
        part = part.strip()
        if part == "...":
          data["variable_arguments"] = True
        else:
          arg = parse_argument(part)
          if not arg:
            argument_failed = True
            break
          data["arguments"].append(arg)
      if argument_failed:
        print "Unsupported type encountered. Method is skipped:\n%s\n" % signature_string
        continue
      if signature_string_parts2[1].strip() == "const":
        data["const"] = True

      if data["name"].startswith("operator"):
        data["operator"] = data["name"][len("operator"):].strip()
      if class_name:
        if data["name"] == class_name:
          data["constructor"] = True
          if "return_type" in data:
            raise Exception("Constructors are not allowed to have return types")
        elif data["name"] == "~" + class_name:
          data["destructor"] = True
          if "return_type" in data:
            raise Exception("Destructors are not allowed to have return types")
        elif "operator" in data:
          pass
          # operators may or may not have return type
        else: # not constructor
          if not "return_type" in data:
            raise Exception("No return type in a method")

    result.append(data)
  return result



def parse_section(soup, class_name, id, attrs):
  header = soup.find("h2", id=id)
  if not header: return []
  return parse_methods(header.findNext("table"), class_name, attrs)


def parse_macros(soup, id):
  macros_header = soup.find("h2", id=id)
  if not macros_header: return None
  macros = []
  for row in macros_header.findNext("table").findAll("tr"):
    macros.append(row.text)
  return macros



def parse_doc(filename):
  data = open(filename,'r').read()
  soup = BeautifulSoup(data, convertEntities=BeautifulSoup.HTML_ENTITIES)
  result = {}
  title = soup.find("h1", { "class": "title" })
  if not title: raise Exception("Unknown HTML layout")

  if title.text.startswith("<Qt"):
    result["type"] = "global"
    macros = parse_macros(soup, "Macrosx")
    if macros: result["macros"] = macros
    end_index = title.text.find(">")
    if not end_index:
      raise Exception("Invalid global header title")
    result["include_file"] = title.text[1:end_index]
    result["methods"] = parse_section(soup, None, "Functions", {})
    return result

  title_parts = title.text.split(' ')
  if len(title_parts) == 2 and title_parts[1].strip() == "Class":
    result["type"] = "class"
    class_without_namespace = title_parts[0].strip()
    result["include_file"] = class_without_namespace
    full_class_tag = soup.find("span", { "class": "small-subtitle" })
    if full_class_tag:
      result["class"] = full_class_tag.text.strip(" ()")
    else:
      result["class"] = class_without_namespace

    if not result["class"].startswith("Q"):
      print "Warning: %s: class %s doesn't start with Q" % (filename, result["class"])
  else:
    print "Unsupported title value in %s" % filename
    return None


  result["methods"] = []
  result["methods"] += parse_section(soup, class_without_namespace, "public-functions", {})
  result["methods"] += parse_section(soup, class_without_namespace, "protected-functions", { "protected": True })
  result["methods"] += parse_section(soup, class_without_namespace, "public-slots", { "slot": True })
  result["methods"] += parse_section(soup, class_without_namespace, "protected-slots", { "slot": True, "protected": True })
  result["methods"] += parse_section(soup, class_without_namespace, "static-public-members", { "static": True })
  result["methods"] += parse_section(soup, class_without_namespace, "static-protected-members", { "static": True, "protected": True })
  result["methods"] += parse_section(soup, class_without_namespace, "signals", { "signal": True })
  result["methods"] += parse_section(soup, None, "related-non-members", {})
  macros = parse_macros(soup, "macros")
  if macros: result["macros"] = macros

  return result

#pp = pprint.PrettyPrinter()


def parse(input_folder):
  all_data = []
  #for filename in ['../qt-doc/html/qtendian.html']:
  for filename in glob.iglob(input_folder + '/*.html'):
    bad_endings = "members", "obsolete", "compat", "example", "pro", "cpp", "h", "ui"
    bad = False
    for ending in bad_endings:
      if filename.endswith("-%s.html" % ending):
        bad = True
        break
    if bad: continue

    #print filename
    result = parse_doc(filename)
    if result:
      all_data.append(result)
    #pp.pprint(result)
  return all_data

if len(sys.argv) < 3:
  print "Usage: parser doc_html_folder output_filename"
else:
  print "Parsing documentation..."
  parse_result = parse(sys.argv[1])
  with open(sys.argv[2], "w") as f:
    json.dump(parse_result, f, indent=2)

